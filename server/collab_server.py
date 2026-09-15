#!/usr/bin/env python3
"""
LibreOffice Collab — servidor em Python / Python host server.

PT: A alternativa "basta correr um script" ao collab-server.exe. Faz o mesmo
    que o servidor em Rust (lista os documentos da pasta partilhada e fala WOPI
    com o Collabora Online) e ainda:
      - arranca o motor Collabora com o Docker, se estiver instalado;
      - abre a porta na firewall do Windows (pede permissoes uma vez);
      - serve uma pagina web, para quem nao tem a aplicacao instalada abrir
        os documentos no browser a partir de qualquer PC da rede.

EN: The "just run a script" alternative to collab-server.exe. Does what the
    Rust server does (lists the shared folder and speaks WOPI to Collabora
    Online) and also:
      - starts the Collabora engine through Docker when it is installed;
      - opens the Windows Firewall port (asks for permission once);
      - serves a web page so people without the app can open the documents
        in a browser from any PC on the network.

Uso / Usage:
    python collab_server.py                       # pasta e porta por omissao
    python collab_server.py --dir "D:\\Docs" --port 7373
    python collab_server.py --cool http://192.168.1.50:9980 --no-docker
    python collab_server.py --help

So usa a biblioteca padrao. Se o pacote `zeroconf` estiver instalado
(`pip install zeroconf`) o servidor tambem se anuncia por mDNS e aparece
sozinho na aplicacao de secretaria, tal como o servidor em Rust.
Standard library only. With the optional `zeroconf` package installed the
server also advertises itself over mDNS, exactly like the Rust server does.
"""

import argparse
import ctypes
import json
import locale
import os
import platform
import secrets
import shutil
import socket
import string
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import webbrowser
from datetime import datetime, timezone
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, quote, urlsplit

APP_VERSION = "1.0.0"
GITHUB_OWNER = "Cmprfda"
GITHUB_REPO = "libreoffice-collab"

HERE = Path(__file__).resolve().parent
SERVICE_TYPE = "_locollab._tcp.local."
COOL_PORT = 9980
MDNS_PORT = 5353
TOKEN_TTL_SECONDS = 12 * 60 * 60
# Collabora uploads the whole document on every save; 256 MB is generous
# enough for a presentation full of images and still bounded.
MAX_UPLOAD_BYTES = 256 * 1024 * 1024

# a consola do Windows e cp1252: sem isto, imprimir um caractere fora dessa
# tabela rebenta o servidor a meio de um log
for _stream in (sys.stdout, sys.stderr):
    try:
        # line_buffering: the log stays readable when stdout is a pipe (launcher)
        _stream.reconfigure(errors="replace", line_buffering=True)
    except (AttributeError, OSError):
        pass

# ---------------------------------------------------------------- documents --

# Extensions offered for editing. Anything else in the folder is ignored so
# stray PDFs and images do not clutter the list. Same table as server/src/docs.rs.
EDITABLE = {
    "odt": "text", "ott": "text", "doc": "text", "docx": "text", "rtf": "text", "txt": "text",
    "ods": "spreadsheet", "ots": "spreadsheet", "xls": "spreadsheet", "xlsx": "spreadsheet",
    "csv": "spreadsheet",
    "odp": "presentation", "otp": "presentation", "ppt": "presentation", "pptx": "presentation",
    "odg": "drawing",
}


def encode_id(file_name):
    """Document ids are the hex-encoded file name: reversible, URL-safe and
    unable to express `..` or a separator once validated by `resolve`."""
    return file_name.encode("utf-8").hex()


def decode_id(doc_id):
    if not doc_id or len(doc_id) % 2:
        return None
    try:
        return bytes.fromhex(doc_id).decode("utf-8")
    except ValueError:
        return None


def safe_name(name):
    """A bare file name that stays inside the shared folder, or None."""
    if not name or any(c in name for c in "/\\:") or ".." in name:
        return None
    if name.startswith((".~lock", "~$")) or name.strip() != name:
        return None
    ext = name.rsplit(".", 1)[-1].lower() if "." in name else ""
    if ext not in EDITABLE:
        return None
    return name


def list_documents(docs_dir):
    """Every editable file in the shared folder, newest first."""
    documents = []
    try:
        entries = list(os.scandir(docs_dir))
    except OSError:
        return documents
    for entry in entries:
        try:
            if not entry.is_file():
                continue
            stat = entry.stat()
        except OSError:
            continue
        name = entry.name
        # Skip LibreOffice lock files and Office temp files if a desktop app
        # ever touched the folder by accident.
        if name.startswith((".~lock", "~$")):
            continue
        ext = name.rsplit(".", 1)[-1].lower() if "." in name else ""
        kind = EDITABLE.get(ext)
        if not kind:
            continue
        documents.append({
            "id": encode_id(name),
            "name": name,
            "extension": ext,
            "kind": kind,
            "size": stat.st_size,
            "modified": int(stat.st_mtime),
        })
    documents.sort(key=lambda d: d["modified"], reverse=True)
    return documents


def resolve(docs_dir, doc_id):
    """Maps a document id back to (path, name) inside the shared folder, or
    None for anything that would escape it."""
    name = decode_id(doc_id)
    if not name or any(c in name for c in "/\\:") or ".." in name:
        return None
    path = Path(docs_dir) / name
    if not path.is_file():
        return None
    return path, name


def write_atomic(path, data):
    """Writes next to the target and renames, so a crash mid-save never leaves
    a truncated document behind."""
    path = Path(path)
    temp = path.with_name(path.name + ".collab-tmp")
    with open(temp, "wb") as fh:
        fh.write(data)
    os.replace(temp, path)


def version_of(path):
    try:
        stat = os.stat(path)
        return f"{int(stat.st_mtime)}_{stat.st_size}"
    except OSError:
        return "0_0"


def rfc3339(unix_seconds):
    return datetime.fromtimestamp(unix_seconds, timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def ensure_dir(docs_dir):
    """Creates the shared folder on first run and drops a short readme in it."""
    docs_dir = Path(docs_dir)
    docs_dir.mkdir(parents=True, exist_ok=True)
    readme = docs_dir / "LEIA-ME.txt"
    if not readme.exists():
        readme.write_text(
            "Coloque aqui os documentos a partilhar (.odt, .ods, .odp, .docx, .xlsx, .pptx).\r\n"
            "Todos os utilizadores da aplicacao LibreOffice Collab veem estes ficheiros.\r\n"
            "\r\n"
            "Place the documents you want to share in this folder.\r\n"
            "Everyone running LibreOffice Collab will see them.\r\n",
            encoding="utf-8",
        )


# ----------------------------------------------------------------- sessions --

class Sessions:
    """Short-lived WOPI access tokens, in memory only. Restarting the server
    invalidates them, which is the right behaviour for a LAN tool."""

    def __init__(self):
        self._lock = threading.Lock()
        self._tokens = {}

    def create(self, doc_id, user):
        alphabet = string.ascii_letters + string.digits
        token = "".join(secrets.choice(alphabet) for _ in range(40))
        now = int(time.time())
        with self._lock:
            # Opportunistic cleanup keeps the map from growing forever.
            self._tokens = {t: s for t, s in self._tokens.items() if s["expires_at"] > now}
            self._tokens[token] = {"doc_id": doc_id, "user": user,
                                   "expires_at": now + TOKEN_TTL_SECONDS}
        return token

    def validate(self, token, doc_id):
        """The session when the token is valid *for this document*. Binding the
        token to the document matters: otherwise a token for a harmless file
        would grant write access to every other one."""
        with self._lock:
            session = self._tokens.get(token)
        if not session or session["doc_id"] != doc_id or session["expires_at"] <= time.time():
            return None
        return session


# ------------------------------------------------------------------- config --

def lan_ip():
    """The address this machine uses to reach the rest of the LAN. Connecting a
    UDP socket sends nothing; it only makes the OS pick a route."""
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except OSError:
        return "127.0.0.1"


def default_docs_dir():
    public = os.environ.get("PUBLIC")
    if public:
        return Path(public) / "Documentos Partilhados"
    return Path("documentos")


def machine_name():
    return os.environ.get("COMPUTERNAME") or os.environ.get("HOSTNAME") or "LibreOffice Collab"


def console_language():
    """'pt' or 'en', from the Windows UI language (falls back to Portuguese,
    like the rest of the project)."""
    try:
        if platform.system() == "Windows":
            lang_id = ctypes.windll.kernel32.GetUserDefaultUILanguage()
            return "pt" if (lang_id & 0xFF) == 0x16 else "en"
        code = locale.getlocale()[0] or ""
        return "pt" if code.lower().startswith("pt") else "en"
    except Exception:
        return "pt"


class Config:
    def __init__(self, args):
        self.public_ip = lan_ip()
        self.docs_dir = Path(args.dir or os.environ.get("COLLAB_DIR") or default_docs_dir())
        self.port = int(args.port or os.environ.get("COLLAB_PORT") or 7373)
        self.name = args.name or os.environ.get("COLLAB_NAME") or machine_name()
        self.bind_host = args.host
        cool = args.cool or os.environ.get("COLLAB_COOL_URL") or f"http://{self.public_ip}:{COOL_PORT}"
        self.cool_url = cool.rstrip("/")
        self.lang = args.lang or console_language()

    @property
    def base_url(self):
        # Clients and Collabora reach us through the LAN address, never through
        # the bind address (0.0.0.0 means nothing to them).
        return f"http://{self.public_ip}:{self.port}"

    def cool_is_local(self):
        host = urlsplit(self.cool_url).hostname or ""
        return host in ("localhost", "127.0.0.1", self.public_ip)


# ------------------------------------------------------------------- docker --

def run(cmd, **kwargs):
    kwargs.setdefault("stdout", subprocess.DEVNULL)
    kwargs.setdefault("stderr", subprocess.DEVNULL)
    try:
        return subprocess.run(cmd, **kwargs).returncode
    except (OSError, subprocess.SubprocessError):
        return None


def find_compose_file(say):
    """The compose file from the repository checkout when we are running from
    one, otherwise a copy kept (and downloaded if needed) under %LocalAppData%."""
    local = HERE.parent / "docker" / "docker-compose.yml"
    if local.is_file():
        return local
    base = Path(os.environ.get("LOCALAPPDATA", tempfile.gettempdir())) / "Programs" / "LibreOffice Collab Server"
    target = base / "docker" / "docker-compose.yml"
    if target.is_file():
        return target
    target.parent.mkdir(parents=True, exist_ok=True)
    url = f"https://raw.githubusercontent.com/{GITHUB_OWNER}/{GITHUB_REPO}/main/docker/docker-compose.yml"
    say("A descarregar docker-compose.yml...", "Downloading docker-compose.yml...")
    try:
        with urllib.request.urlopen(url, timeout=20) as response:
            target.write_bytes(response.read())
    except (urllib.error.URLError, OSError) as exc:
        say(f"Nao consegui descarregar o ficheiro compose ({exc}).",
            f"Could not download the compose file ({exc}).")
        return None
    return target


def start_collabora(config, say):
    """`docker compose up -d` for the Collabora engine. Returns True when the
    container was (re)started, False when Docker is not usable."""
    if not shutil.which("docker"):
        say("Docker Desktop nao esta instalado: o motor Collabora nao vai arrancar.\n"
            "  Instale-o em https://www.docker.com/products/docker-desktop/ ou passe\n"
            "  --cool http://<outro-pc>:9980 se o Collabora ja correr noutro lado.",
            "Docker Desktop is not installed: the Collabora engine will not start.\n"
            "  Install it from https://www.docker.com/products/docker-desktop/ or pass\n"
            "  --cool http://<other-pc>:9980 if Collabora already runs elsewhere.")
        return False
    if run(["docker", "info"], timeout=30) != 0:
        say("O Docker Desktop esta instalado mas nao esta a correr. Abra-o, espere que\n"
            "  o icone fique verde e volte a correr este script.",
            "Docker Desktop is installed but not running. Start it, wait for the icon to\n"
            "  turn green and run this script again.")
        return False

    compose = find_compose_file(say)
    if not compose:
        return False
    # docker compose reads .env from the compose file's directory. HOST_IP must
    # match the address we advertise or every document opens with "Access denied".
    (compose.parent / ".env").write_text(
        f"HOST_IP={config.public_ip}\nCOLLAB_PORT={config.port}\nCOOL_ADMIN_PASSWORD=collab\n",
        encoding="utf-8",
    )
    say("A arrancar o motor de colaboracao (Collabora). Na primeira vez demora alguns minutos...",
        "Starting the collaboration engine (Collabora). The first run takes a few minutes...")
    code = run(["docker", "compose", "up", "-d"], cwd=str(compose.parent), stdout=None, stderr=None)
    if code != 0:
        say(f"docker compose falhou (codigo {code}).", f"docker compose failed (exit {code}).")
        return False
    return True


def cool_ready(cool_url, timeout=2.0):
    try:
        with urllib.request.urlopen(f"{cool_url}/hosting/discovery", timeout=timeout) as response:
            return response.status == 200
    except (urllib.error.URLError, OSError, ValueError):
        return False


def watch_collabora(config, say):
    """Background: says when Collabora answers, so the user knows when the
    first document can be opened."""
    deadline = time.time() + 10 * 60
    while time.time() < deadline:
        if cool_ready(config.cool_url):
            say(f"[ok] Collabora pronto em {config.cool_url}",
                f"[ok] Collabora ready at {config.cool_url}")
            return
        time.sleep(3)
    say("[aviso] O Collabora ainda nao responde. Veja o Docker Desktop (contentor libreoffice-collab-code).",
        "[warning] Collabora is still not answering. Check Docker Desktop (container libreoffice-collab-code).")


# ----------------------------------------------------------------- firewall --

FIREWALL_RULES = (
    ("LibreOffice Collab (WOPI)", "TCP", None),
    ("LibreOffice Collab (Collabora)", "TCP", COOL_PORT),
    ("LibreOffice Collab (mDNS)", "UDP", MDNS_PORT),
)


def is_admin():
    try:
        return bool(ctypes.windll.shell32.IsUserAnAdmin())
    except Exception:
        return False


def firewall_rule_exists(name):
    return run(["netsh", "advfirewall", "firewall", "show", "rule", f"name={name}"]) == 0


def netsh_add(name, protocol, port):
    # Private and Domain profiles only: this is an office LAN tool.
    return (f'netsh advfirewall firewall add rule name="{name}" dir=in action=allow '
            f'protocol={protocol} localport={port} profile=private,domain')


def open_firewall(config, say):
    """Creates the inbound rules once. Without admin rights it asks for them
    through the normal Windows prompt; declining just leaves the rules absent."""
    if platform.system() != "Windows":
        return
    missing = [(name, proto, port or config.port) for name, proto, port in FIREWALL_RULES
               if not firewall_rule_exists(name)]
    if not missing:
        return
    say("A abrir as portas na firewall do Windows...", "Opening the Windows Firewall ports...")
    if is_admin():
        for name, proto, port in missing:
            run(netsh_add(name, proto, port), shell=True)
    else:
        # One elevated cmd for all the rules, so there is a single UAC prompt.
        script = Path(tempfile.gettempdir()) / "libreoffice-collab-firewall.cmd"
        script.write_text("@echo off\r\n" + "\r\n".join(netsh_add(*rule) for rule in missing) + "\r\n",
                          encoding="ascii")
        run(["powershell", "-NoProfile", "-Command",
             f"Start-Process -FilePath '{script}' -Verb RunAs -Wait -WindowStyle Hidden"], timeout=120)
    still_missing = [name for name, _, _ in missing if not firewall_rule_exists(name)]
    if still_missing:
        say("Sem permissoes de administrador: as regras de firewall nao foram criadas.\n"
            "  Os outros PCs podem nao conseguir ligar-se. Corra uma vez como administrador,\n"
            "  ou passe --no-firewall para nao voltar a perguntar.",
            "No administrator rights: the firewall rules were not created.\n"
            "  Other PCs may fail to connect. Run once as administrator,\n"
            "  or pass --no-firewall to stop asking.")


# --------------------------------------------------------------------- mDNS --

def advertise_mdns(config, say):
    """Optional: with `zeroconf` installed the desktop app finds this server on
    its own. Returns the objects to keep alive, or None."""
    try:
        from zeroconf import ServiceInfo, Zeroconf
    except ImportError:
        say("mDNS desligado (opcional): `pip install zeroconf` faz o servidor aparecer sozinho\n"
            "  na aplicacao. Sem isso, escreva o endereco em Definicoes -> Servidor.",
            "mDNS off (optional): `pip install zeroconf` makes this server show up on its own\n"
            "  in the app. Without it, type the address in Settings -> Server.")
        return None
    instance = config.name.replace(".", "-").strip() or "LibreOffice-Collab"
    info = ServiceInfo(
        SERVICE_TYPE,
        f"{instance}.{SERVICE_TYPE}",
        addresses=[socket.inet_aton(config.public_ip)],
        port=config.port,
        properties={"name": config.name, "version": APP_VERSION,
                    "cool": config.cool_url, "scheme": "http"},
        server=f"{instance.lower()}.local.",
    )
    try:
        zc = Zeroconf()
        zc.register_service(info)
        return zc, info
    except Exception as exc:
        say(f"[aviso] mDNS indisponivel ({exc}).", f"[warning] mDNS unavailable ({exc}).")
        return None


# ---------------------------------------------------------------------- HTTP --

def urlencode(value):
    return quote(str(value), safe="-_.~")


class Handler(BaseHTTPRequestHandler):
    server_version = f"collab-server-py/{APP_VERSION}"
    config = None       # set by main()
    sessions = None
    _cool_cache = (0.0, False)

    # -- plumbing ----------------------------------------------------------

    def log_message(self, fmt, *args):
        # The desktop app polls health every 5 s and documents every 15 s;
        # logging those would bury the saves, which are what matter.
        if self.path.startswith(("/api/health", "/api/documents")):
            return
        sys.stdout.write("[http] %s %s\n" % (self.command, self.path))

    def send_bytes(self, status, body, content_type="application/octet-stream", headers=()):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        for key, value in headers:
            self.send_header(key, value)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def send_json(self, payload, status=HTTPStatus.OK, headers=()):
        self.send_bytes(status, json.dumps(payload).encode("utf-8"),
                        "application/json; charset=utf-8", headers)

    def fail(self, status, message=""):
        self.send_bytes(status, (message or status.phrase).encode("utf-8"), "text/plain; charset=utf-8")

    def read_body(self):
        length = int(self.headers.get("Content-Length") or 0)
        if length > MAX_UPLOAD_BYTES:
            return None
        return self.rfile.read(length) if length else b""

    def query_value(self, query, key, default=""):
        values = query.get(key)
        return values[0] if values else default

    def route(self):
        url = urlsplit(self.path)
        parts = [p for p in url.path.split("/") if p]
        return parts, parse_qs(url.query)

    # -- verbs -------------------------------------------------------------

    def do_GET(self):
        parts, query = self.route()
        if not parts:
            return self.page_index()
        if parts == ["api", "health"]:
            return self.api_health()
        if parts == ["api", "documents"]:
            return self.send_json(list_documents(self.config.docs_dir))
        if len(parts) == 3 and parts[:2] == ["api", "session"]:
            return self.api_session(parts[2], query)
        if len(parts) == 2 and parts[0] == "open":
            return self.page_open(parts[1], query)
        if len(parts) == 3 and parts[:2] == ["wopi", "files"]:
            return self.wopi_check_file_info(parts[2], query)
        if len(parts) == 4 and parts[:2] == ["wopi", "files"] and parts[3] == "contents":
            return self.wopi_get_file(parts[2], query)
        self.fail(HTTPStatus.NOT_FOUND)

    do_HEAD = do_GET

    def do_POST(self):
        parts, query = self.route()
        if parts == ["api", "upload"]:
            return self.api_upload(query)
        if len(parts) == 3 and parts[:2] == ["wopi", "files"]:
            return self.wopi_lock_noop(parts[2], query)
        if len(parts) == 4 and parts[:2] == ["wopi", "files"] and parts[3] == "contents":
            return self.wopi_put_file(parts[2], query)
        self.fail(HTTPStatus.NOT_FOUND)

    # -- REST API (same shapes as server/src/api.rs) -----------------------

    def cool_status(self):
        checked, ready = Handler._cool_cache
        if time.time() - checked > 10:
            ready = cool_ready(self.config.cool_url)
            Handler._cool_cache = (time.time(), ready)
        return ready

    def api_health(self):
        self.send_json({
            "ok": True,
            "name": self.config.name,
            "version": APP_VERSION,
            "cool_url": self.config.cool_url,
            "documents": len(list_documents(self.config.docs_dir)),
            # Extra, ignored by the desktop app: lets the web page say whether
            # the engine is up yet.
            "cool_ready": self.cool_status(),
            "python": True,
        })

    def editor_url_for(self, doc_id, query):
        """Mints a WOPI token and builds the Collabora URL, or None if the
        document vanished between listing and opening."""
        if not resolve(self.config.docs_dir, doc_id):
            return None
        user = self.query_value(query, "user").strip() or "Utilizador"
        lang = self.query_value(query, "lang") or "pt-PT"
        token = self.sessions.create(doc_id, user)
        wopi_src = f"{self.config.base_url}/wopi/files/{doc_id}"
        editor_url = (f"{self.config.cool_url}/browser/dist/cool.html"
                      f"?WOPISrc={urlencode(wopi_src)}&access_token={urlencode(token)}&lang={urlencode(lang)}")
        return editor_url, wopi_src

    def api_session(self, doc_id, query):
        result = self.editor_url_for(doc_id, query)
        if not result:
            return self.fail(HTTPStatus.NOT_FOUND)
        editor_url, wopi_src = result
        self.send_json({"editor_url": editor_url, "wopi_src": wopi_src})

    def api_upload(self, query):
        """Browser users cannot copy into the host's folder, so the web page
        lets them drop a file here instead. Only editable types, never a path."""
        name = safe_name(self.query_value(query, "name"))
        if not name:
            return self.fail(HTTPStatus.BAD_REQUEST, "invalid file name")
        target = Path(self.config.docs_dir) / name
        if target.exists() and self.query_value(query, "overwrite") != "1":
            return self.fail(HTTPStatus.CONFLICT, "exists")
        body = self.read_body()
        if body is None:
            return self.fail(HTTPStatus.REQUEST_ENTITY_TOO_LARGE)
        if not body:
            return self.fail(HTTPStatus.BAD_REQUEST, "empty file")
        try:
            write_atomic(target, body)
        except OSError as exc:
            return self.fail(HTTPStatus.INTERNAL_SERVER_ERROR, str(exc))
        print(f"[upload] {name} ({len(body)} bytes)")
        self.send_json({"ok": True, "id": encode_id(name), "name": name})

    # -- WOPI (same behaviour as server/src/wopi.rs) -----------------------

    def authorise(self, doc_id, query):
        session = self.sessions.validate(self.query_value(query, "access_token"), doc_id)
        if not session:
            self.fail(HTTPStatus.UNAUTHORIZED)
            return None
        resolved = resolve(self.config.docs_dir, doc_id)
        if not resolved:
            self.fail(HTTPStatus.NOT_FOUND)
            return None
        path, name = resolved
        return path, name, session["user"]

    def wopi_check_file_info(self, doc_id, query):
        auth = self.authorise(doc_id, query)
        if not auth:
            return
        path, name, user = auth
        try:
            stat = os.stat(path)
        except OSError:
            return self.fail(HTTPStatus.NOT_FOUND)
        modified = int(stat.st_mtime)
        self.send_json({
            "BaseFileName": name,
            "Size": stat.st_size,
            "OwnerId": "collab-server",
            # One WOPI user per person, so Collabora shows real names and gives
            # each their own cursor colour.
            "UserId": user,
            "UserFriendlyName": user,
            "UserCanWrite": True,
            # No PutRelativeFile ("Save as"), so say so up front.
            "UserCanNotWriteRelative": True,
            "SupportsUpdate": True,
            "SupportsLocks": False,
            "SupportsRename": False,
            "DisablePrint": False,
            "DisableExport": False,
            "DisableCopy": False,
            "HideUserList": "",
            "LastModifiedTime": rfc3339(modified),
            # Must change whenever the bytes change, or Collabora serves a
            # stale document from its cache.
            "Version": f"{modified}_{stat.st_size}",
        })

    def wopi_get_file(self, doc_id, query):
        auth = self.authorise(doc_id, query)
        if not auth:
            return
        path, _, _ = auth
        try:
            data = path.read_bytes()
        except OSError:
            return self.fail(HTTPStatus.INTERNAL_SERVER_ERROR)
        self.send_bytes(HTTPStatus.OK, data, headers=[("X-WOPI-ItemVersion", version_of(path))])

    def wopi_put_file(self, doc_id, query):
        auth = self.authorise(doc_id, query)
        if not auth:
            return
        path, name, user = auth
        body = self.read_body()
        if body is None:
            return self.fail(HTTPStatus.REQUEST_ENTITY_TOO_LARGE)
        if not body:
            # Never truncate a document because of an empty autosave.
            return self.fail(HTTPStatus.BAD_REQUEST)
        try:
            write_atomic(path, body)
        except OSError as exc:
            print(f"[wopi] failed to save {name}: {exc}", file=sys.stderr)
            return self.fail(HTTPStatus.INTERNAL_SERVER_ERROR)
        reason = (self.headers.get("X-LOOL-WOPI-Timestamp")
                  or self.headers.get("X-COOL-WOPI-Timestamp") or "autosave")
        print(f"[wopi] saved {name} ({len(body)} bytes) by {user} [{reason}]")
        self.send_json({"LastModifiedTime": rfc3339(int(time.time()))},
                       headers=[("X-WOPI-ItemVersion", version_of(path))])

    def wopi_lock_noop(self, doc_id, query):
        """LOCK / UNLOCK / REFRESH_LOCK: Collabora is the only writer, so there
        is nobody to lock out. Acknowledging keeps the protocol happy."""
        auth = self.authorise(doc_id, query)
        if not auth:
            return
        path, _, _ = auth
        self.send_bytes(HTTPStatus.OK, b"", "text/plain",
                        headers=[("X-WOPI-Lock", self.headers.get("X-WOPI-Lock", "")),
                                 ("X-WOPI-ItemVersion", version_of(path))])

    # -- web page -----------------------------------------------------------

    def page_index(self):
        html = (INDEX_HTML.replace("__NAME__", json.dumps(self.config.name))
                .replace("__LANG__", json.dumps(self.config.lang))
                .replace("__VERSION__", APP_VERSION))
        self.send_bytes(HTTPStatus.OK, html.encode("utf-8"), "text/html; charset=utf-8")

    def page_open(self, doc_id, query):
        """A plain link target: mints the session and sends the browser straight
        to Collabora, so the page needs no JavaScript to open a document."""
        result = self.editor_url_for(doc_id, query)
        if not result:
            return self.fail(HTTPStatus.NOT_FOUND)
        self.send_response(HTTPStatus.FOUND)
        self.send_header("Location", result[0])
        self.send_header("Content-Length", "0")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()


INDEX_HTML = r"""<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>LibreOffice Collab</title>
<style>
  :root { --bg:#f4f5f7; --card:#fff; --text:#1c1e21; --muted:#66707a; --line:#e2e5e9;
          --accent:#1a73e8; --ok:#1e8e3e; --warn:#c5221f; --hover:#f0f4ff; }
  @media (prefers-color-scheme: dark) {
    :root { --bg:#16181c; --card:#1f2228; --text:#e8eaed; --muted:#9aa0a6; --line:#30343b;
            --accent:#8ab4f8; --ok:#5bb974; --warn:#f28b82; --hover:#262b36; }
  }
  * { box-sizing:border-box; }
  body { margin:0; padding:24px 16px; background:var(--bg); color:var(--text);
         font:15px/1.45 system-ui, "Segoe UI", sans-serif; }
  main { max-width:820px; margin:0 auto; }
  header { display:flex; flex-wrap:wrap; gap:12px 20px; align-items:baseline; justify-content:space-between; }
  h1 { font-size:22px; margin:0; } h1 small { color:var(--muted); font-weight:normal; font-size:14px; margin-left:8px; }
  .badge { display:inline-flex; align-items:center; gap:6px; font-size:13px; color:var(--muted); }
  .dot { width:9px; height:9px; border-radius:50%; background:var(--warn); }
  .dot.ok { background:var(--ok); }
  .bar { display:flex; flex-wrap:wrap; gap:10px; align-items:center; margin:20px 0 14px; }
  label { color:var(--muted); font-size:13px; }
  input[type=text] { padding:8px 10px; border:1px solid var(--line); border-radius:8px; background:var(--card);
          color:var(--text); font:inherit; min-width:200px; }
  button, .btn { padding:8px 14px; border:1px solid var(--line); border-radius:8px; background:var(--card);
          color:var(--text); font:inherit; cursor:pointer; }
  button:hover, .btn:hover { background:var(--hover); }
  .spacer { flex:1; }
  .card { background:var(--card); border:1px solid var(--line); border-radius:12px; overflow:hidden; }
  .doc { display:flex; gap:14px; align-items:center; padding:12px 16px; border-top:1px solid var(--line);
         color:inherit; text-decoration:none; }
  .doc:first-child { border-top:0; } .doc:hover { background:var(--hover); }
  .icon { width:38px; height:38px; border-radius:8px; display:grid; place-items:center; font-weight:700;
          font-size:12px; color:#fff; flex:none; }
  .text { background:#2b7bd6; } .spreadsheet { background:#1e8e3e; } .presentation { background:#d9781d; } .drawing { background:#8e44ad; }
  .name { font-weight:600; overflow-wrap:anywhere; } .meta { color:var(--muted); font-size:13px; }
  .grow { flex:1; min-width:0; }
  .empty { padding:36px 16px; text-align:center; color:var(--muted); }
  .drop { border:2px dashed var(--line); border-radius:12px; padding:14px; text-align:center; color:var(--muted);
          margin-top:14px; font-size:13px; }
  .drop.over { border-color:var(--accent); color:var(--accent); }
  footer { margin-top:22px; color:var(--muted); font-size:12px; }
  select { padding:6px 8px; border:1px solid var(--line); border-radius:8px; background:var(--card); color:var(--text); font:inherit; }
</style>
</head>
<body>
<main>
  <header>
    <h1>LibreOffice Collab <small id="server"></small></h1>
    <span class="badge"><span class="dot" id="dot"></span><span id="status"></span></span>
  </header>

  <div class="bar">
    <label for="user" data-t="yourName"></label>
    <input type="text" id="user" maxlength="60" autocomplete="name">
    <span class="spacer"></span>
    <select id="lang"><option value="pt">Português</option><option value="en">English</option></select>
    <button id="refresh" data-t="refresh"></button>
  </div>

  <div class="card" id="list"></div>

  <div class="drop" id="drop">
    <span data-t="dropHint"></span>
    <label class="btn" style="margin-left:8px"><span data-t="choose"></span>
      <input type="file" id="file" hidden multiple accept=".odt,.ott,.doc,.docx,.rtf,.txt,.ods,.ots,.xls,.xlsx,.csv,.odp,.otp,.ppt,.pptx,.odg"></label>
  </div>

  <footer data-t="footer"></footer>
</main>

<script>
const SERVER = __NAME__;
const DEFAULT_LANG = __LANG__;
const T = {
  pt: { yourName:"O seu nome", refresh:"Atualizar", connected:"Ligado", engineDown:"Motor Collabora a arrancar…",
        offline:"Servidor indisponível", noDocs:"Ainda não há documentos partilhados",
        noDocsHint:"Copie ficheiros para a pasta partilhada do servidor, ou largue-os aqui.",
        dropHint:"Largue aqui um documento para o partilhar, ou", choose:"escolha um ficheiro",
        modified:"Modificado", types:{text:"Documento de texto", spreadsheet:"Folha de cálculo",
        presentation:"Apresentação", drawing:"Desenho"}, exists:"Já existe um ficheiro com o nome “{n}”. Substituir?",
        uploadFail:"Não foi possível enviar “{n}”: {e}", badType:"“{n}” não é um tipo de documento suportado.",
        footer:"Todos veem os mesmos ficheiros e podem editá-los ao mesmo tempo. Nada sai da rede local. · v__VERSION__",
        locale:"pt-PT" },
  en: { yourName:"Your name", refresh:"Refresh", connected:"Connected", engineDown:"Collabora engine starting…",
        offline:"Server unavailable", noDocs:"No shared documents yet",
        noDocsHint:"Copy files into the server's shared folder, or drop them here.",
        dropHint:"Drop a document here to share it, or", choose:"choose a file",
        modified:"Modified", types:{text:"Text document", spreadsheet:"Spreadsheet",
        presentation:"Presentation", drawing:"Drawing"}, exists:"A file named “{n}” already exists. Replace it?",
        uploadFail:"Could not upload “{n}”: {e}", badType:"“{n}” is not a supported document type.",
        footer:"Everyone sees the same files and can edit them at the same time. Nothing leaves the local network. · v__VERSION__",
        locale:"en-GB" },
};
const ABBR = { text:"DOC", spreadsheet:"CALC", presentation:"PRES", drawing:"DRAW" };
const store = { get(k, d) { try { return localStorage.getItem(k) ?? d; } catch (e) { return d; } },
                set(k, v) { try { localStorage.setItem(k, v); } catch (e) {} } };
let lang = store.get("collab.lang", DEFAULT_LANG);
if (!T[lang]) lang = "pt";
let docs = [];

const $ = (id) => document.getElementById(id);
const t = () => T[lang];
const fmt = (s, v) => s.replace(/\{(\w)\}/g, (_, k) => v[k]);

function applyLang() {
  document.documentElement.lang = t().locale;
  document.querySelectorAll("[data-t]").forEach(el => { el.textContent = t()[el.dataset.t]; });
  $("lang").value = lang;
  render();
}

function fmtSize(b) { return b < 1024 ? b + " B" : b < 1048576 ? (b/1024).toFixed(0) + " KB" : (b/1048576).toFixed(1) + " MB"; }
function fmtDate(s) { return new Date(s * 1000).toLocaleString(t().locale, { dateStyle:"short", timeStyle:"short" }); }
function userName() { return $("user").value.trim(); }
function openUrl(d) {
  return "/open/" + encodeURIComponent(d.id) + "?lang=" + encodeURIComponent(t().locale) +
         "&user=" + encodeURIComponent(userName());
}

function render() {
  const list = $("list");
  if (!docs.length) {
    list.innerHTML = '<div class="empty"><div>' + t().noDocs + '</div><div class="meta">' + t().noDocsHint + '</div></div>';
    return;
  }
  list.innerHTML = docs.map(d => `
    <a class="doc" href="${openUrl(d)}" target="_blank" rel="noopener" data-id="${d.id}">
      <span class="icon ${d.kind}">${ABBR[d.kind] || "DOC"}</span>
      <span class="grow"><div class="name">${escapeHtml(d.name)}</div>
        <div class="meta">${t().types[d.kind] || ""} · ${t().modified} ${fmtDate(d.modified)} · ${fmtSize(d.size)}</div></span>
    </a>`).join("");
}
function escapeHtml(s) { return s.replace(/[&<>"]/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c])); }

async function refresh() {
  try {
    const [health, list] = await Promise.all([fetch("/api/health").then(r => r.json()),
                                              fetch("/api/documents").then(r => r.json())]);
    docs = list;
    $("server").textContent = health.name || SERVER;
    $("dot").className = "dot" + (health.cool_ready ? " ok" : "");
    $("status").textContent = health.cool_ready ? t().connected : t().engineDown;
  } catch (e) {
    $("dot").className = "dot";
    $("status").textContent = t().offline;
  }
  render();
}

async function upload(file, overwrite) {
  const url = "/api/upload?name=" + encodeURIComponent(file.name) + (overwrite ? "&overwrite=1" : "");
  const res = await fetch(url, { method:"POST", body:file });
  if (res.status === 409) {
    if (confirm(fmt(t().exists, { n:file.name }))) return upload(file, true);
    return;
  }
  if (res.status === 400) { alert(fmt(t().badType, { n:file.name })); return; }
  if (!res.ok) alert(fmt(t().uploadFail, { n:file.name, e:res.status + " " + res.statusText }));
}
async function uploadAll(files) { for (const f of files) await upload(f, false); refresh(); }

$("user").value = store.get("collab.user", "");
$("user").addEventListener("input", () => { store.set("collab.user", userName()); render(); });
$("lang").addEventListener("change", (e) => { lang = e.target.value; store.set("collab.lang", lang); applyLang(); });
$("refresh").addEventListener("click", refresh);
$("file").addEventListener("change", (e) => uploadAll([...e.target.files]));
const drop = $("drop");
["dragenter", "dragover"].forEach(ev => drop.addEventListener(ev, e => { e.preventDefault(); drop.classList.add("over"); }));
["dragleave", "drop"].forEach(ev => drop.addEventListener(ev, e => { e.preventDefault(); drop.classList.remove("over"); }));
drop.addEventListener("drop", e => uploadAll([...e.dataTransfer.files]));

applyLang();
refresh();
setInterval(refresh, 15000);
</script>
</body>
</html>
"""


# --------------------------------------------------------------------- main --

def already_running(port):
    """Windows lets a second bind on the port succeed with SO_REUSEADDR, so we
    ask the port instead of trusting the bind."""
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/health", timeout=2) as response:
            return json.loads(response.read().decode("utf-8")).get("ok") is True
    except (urllib.error.URLError, OSError, ValueError):
        return False


def parse_args():
    parser = argparse.ArgumentParser(
        description="LibreOffice Collab - servidor em Python / Python host server",
        epilog="Env: COLLAB_DIR, COLLAB_PORT, COLLAB_COOL_URL, COLLAB_NAME")
    parser.add_argument("--dir", help="pasta partilhada / shared folder "
                        "(default: %%PUBLIC%%\\Documentos Partilhados)")
    parser.add_argument("--port", type=int, help="porta REST + WOPI / REST + WOPI port (default: 7373)")
    parser.add_argument("--cool", help="URL do Collabora Online (default: http://<este-ip>:9980)")
    parser.add_argument("--name", help="nome na lista de servidores / name in the server list "
                        "(default: nome do PC)")
    parser.add_argument("--host", default="0.0.0.0",
                        help="0.0.0.0 = acessivel na rede local; 127.0.0.1 = so neste PC")
    parser.add_argument("--lang", choices=("pt", "en"), help="idioma da pagina web (default: o do Windows)")
    parser.add_argument("--no-docker", action="store_true",
                        help="nao arrancar o Collabora com o Docker / do not start Collabora via Docker")
    parser.add_argument("--no-firewall", action="store_true",
                        help="nao mexer na firewall do Windows / leave the Windows Firewall alone")
    parser.add_argument("--no-browser", action="store_true",
                        help="nao abrir o browser / do not open the browser")
    return parser.parse_args()


def main():
    args = parse_args()
    config = Config(args)

    def say(pt, en):
        print("  " + (pt if config.lang == "pt" else en))

    ensure_dir(config.docs_dir)

    print("=====================================================")
    print(f" LibreOffice Collab - Servidor / Server (Python) v{APP_VERSION}")
    print("=====================================================")
    print(f" Nome / Name        : {config.name} @ {config.public_ip}:{config.port}")
    print(f" Pasta / Folder     : {config.docs_dir}")
    print(f" Collabora Online   : {config.cool_url}")
    print(f" Endereco / Address : {config.base_url}")
    print("-----------------------------------------------------")

    if already_running(config.port):
        say(f"O servidor ja esta a correr em {config.base_url} - a abrir.",
            f"The server is already running at {config.base_url} - opening it.")
        if not args.no_browser:
            webbrowser.open(f"http://localhost:{config.port}")
        return 0

    if not args.no_docker and config.cool_is_local():
        start_collabora(config, say)
    if not args.no_firewall and config.bind_host == "0.0.0.0":
        open_firewall(config, say)

    try:
        ThreadingHTTPServer.allow_reuse_address = False
        server = ThreadingHTTPServer((config.bind_host, config.port), Handler)
    except OSError as exc:
        say(f"Nao consegui escutar em {config.bind_host}:{config.port} ({exc}).",
            f"Could not listen on {config.bind_host}:{config.port} ({exc}).")
        return 1
    server.daemon_threads = True
    Handler.config = config
    Handler.sessions = Sessions()

    mdns = advertise_mdns(config, say)
    threading.Thread(target=watch_collabora, args=(config, say), daemon=True).start()

    print()
    print("  " + "=" * 58)
    say(f"Neste PC:                       http://localhost:{config.port}",
        f"This PC:                        http://localhost:{config.port}")
    say(f"Outros PCs (mesma rede):        {config.base_url}",
        f"Other PCs (same network):       {config.base_url}")
    say(f"Aplicacao de secretaria:        Definicoes -> Servidor -> {config.base_url}",
        f"Desktop app:                    Settings -> Server -> {config.base_url}")
    print("  " + "=" * 58)
    say("Coloque os documentos a partilhar na pasta acima. Ctrl+C para parar.",
        "Put the documents to share in the folder above. Ctrl+C to stop.")
    print()

    if not args.no_browser:
        webbrowser.open(f"http://localhost:{config.port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        if mdns:
            zc, info = mdns
            try:
                zc.unregister_service(info)
                zc.close()
            except Exception:
                pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
