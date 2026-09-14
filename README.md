<div align="center">

# LibreOffice Collab

**Editar documentos LibreOffice em conjunto, em tempo real, na rede do escritório.**
*Edit LibreOffice documents together, in real time, on your office network.*

Windows 11 · Gratuito e de código aberto (MIT) · Português (PT) e English (EN)

</div>

---

## O problema que isto resolve

Quando dois colegas abrem o mesmo ficheiro numa pasta partilhada, o LibreOffice
cria um ficheiro de bloqueio (`.~lock.relatorio.odt#`) e o segundo fica em
**apenas leitura**. Não há forma de contornar isto com o LibreOffice normal.

O LibreOffice Collab usa o motor **Collabora Online**: o documento é aberto uma
única vez, no computador anfitrião, e todos veem e escrevem na mesma cópia — com
os cursores uns dos outros, como no Google Docs, mas dentro da vossa rede e sem
enviar nada para a Internet.

---

# 🇵🇹 Guia em Português

## O que precisa de fazer (resumo)

| Quem | O que faz | Quantas vezes |
| --- | --- | --- |
| **Um computador** do escritório (o "anfitrião") | Corre o `instalar-servidor.bat` | Uma vez |
| **Todos os outros** | Correm o `instalar.bat` | Uma vez |

Depois disso, toda a gente abre a aplicação e clica num documento. Mais nada.

---

## Parte 1 — Instalar a aplicação (todos os computadores)

### Opção A — o mais simples

1. Vá a **[Releases](https://github.com/Cmprfda/libreoffice-collab/releases/latest)**.
2. Descarregue o ficheiro **`instalar.bat`**.
3. **Faça duplo clique** nele.
4. Espere. A aplicação instala-se sozinha e abre no fim.

> Se o Windows mostrar um aviso azul ("O Windows protegeu o seu PC"), clique em
> **Mais informações** → **Executar mesmo assim**. Isto acontece porque o
> programa é gratuito e não tem certificado pago da Microsoft.

### Opção B — uma linha no PowerShell

Se preferir, abra o **PowerShell** e cole isto:

```powershell
irm https://raw.githubusercontent.com/Cmprfda/libreoffice-collab/main/scripts/install.ps1 | iex
```

### O que o instalador faz

- Descarrega a versão mais recente do GitHub.
- Instala em `%LocalAppData%\Programs\LibreOffice Collab` (**não** pede permissões de administrador).
- Cria um atalho no **Ambiente de Trabalho** e no **Menu Iniciar**.
- Abre a aplicação.

---

## Parte 2 — Preparar o computador que partilha os documentos

Isto faz-se **uma única vez**, e **só num computador**: aquele que vai guardar os
ficheiros. Escolha um que esteja normalmente ligado.

1. Instale o **[Docker Desktop](https://www.docker.com/products/docker-desktop/)**
   e abra-o (espere que o ícone fique verde).
2. Descarregue o **`instalar-servidor.bat`** das
   [Releases](https://github.com/Cmprfda/libreoffice-collab/releases/latest).
3. Faça duplo clique. Diga **Sim** quando o Windows pedir permissões.

O script trata de tudo:

- descarrega o servidor,
- arranca o motor Collabora (a primeira vez demora alguns minutos),
- abre as portas necessárias na firewall do Windows,
- cria a pasta partilhada em **`C:\Users\Public\Documentos Partilhados`**,
- configura o arranque automático com o Windows.

**Para partilhar um documento, basta copiá-lo para essa pasta.** Aparece
imediatamente na aplicação de toda a gente.

---

## Parte 3 — Usar no dia a dia

1. Abra o **LibreOffice Collab** (atalho no Ambiente de Trabalho).
2. O servidor do escritório aparece sozinho na lista — não tem de escrever
   endereços nem números. O indicador em cima mostra **Ligado**.
3. Clique num documento. Abre uma janela de edição.
4. Os seus colegas podem abrir o mesmo documento ao mesmo tempo. Vê os cursores
   deles, com o nome, e as alterações aparecem enquanto escrevem.
5. O documento é guardado automaticamente.

### Se o servidor não aparecer

Vá a **Definições** → **Servidor**, desligue *Detetar servidor automaticamente* e
escreva o endereço que o script do servidor mostrou (por exemplo
`http://192.168.1.50:7373`). Clique em **Testar ligação**.

---

## Definições

| Definição | Para que serve |
| --- | --- |
| **Idioma da aplicação** | Português (PT) ou English (EN). Muda na hora, sem reiniciar. |
| **Detetar servidor automaticamente** | Ligado por omissão. Encontra o servidor sozinho. |
| **Endereço do servidor** | Alternativa manual, caso a deteção automática falhe. |
| **Tema** | Do sistema, Claro ou Escuro. |
| **Iniciar minimizado** | A aplicação arranca discretamente na área de notificação. |
| **Procurar Atualizações** | Verifica de imediato se há uma versão nova. |
| **Instalar atualizações automaticamente** | Instala novas versões sozinha, em segundo plano. |

### Atualizações

A aplicação verifica se há novidades ao arrancar e de 6 em 6 horas. Quando há uma
versão nova aparece uma notificação do Windows e uma janela com o botão
**Atualizar e reiniciar**. Cada atualização é verificada com uma assinatura
digital antes de ser instalada — uma versão adulterada é recusada.

---

## Perguntas frequentes

**Os documentos saem da empresa?**
Não. Tudo fica na vossa rede local. A única ligação à Internet é a verificação de
atualizações no GitHub.

**Preciso de ter o LibreOffice instalado?**
Não. A edição acontece no motor Collabora, no computador anfitrião.

**E se o computador anfitrião se desligar?**
Os documentos ficam guardados na pasta partilhada, intactos. Quando voltar a
ligar, tudo volta a funcionar sozinho.

**Que formatos posso usar?**
`.odt`, `.ods`, `.odp`, `.odg`, `.docx`, `.xlsx`, `.pptx`, `.doc`, `.xls`,
`.ppt`, `.rtf`, `.csv`, `.txt`.

---
---

# 🇬🇧 English Guide

## What this fixes

When two colleagues open the same file on a shared drive, LibreOffice creates a
lock file (`.~lock.report.odt#`) and the second person is forced into
**read-only** mode. Plain LibreOffice has no way around this.

LibreOffice Collab uses the **Collabora Online** engine: the document is opened
once, on the host machine, and everyone reads and writes the same copy — seeing
each other's cursors, like Google Docs, but inside your own network with nothing
leaving the building.

## What you need to do (summary)

| Who | What they do | How often |
| --- | --- | --- |
| **One computer** in the office (the "host") | Runs `instalar-servidor.bat` | Once |
| **Everyone else** | Runs `instalar.bat` | Once |

---

## Step 1 — Install the app (every computer)

### Option A — the simple way

1. Go to **[Releases](https://github.com/Cmprfda/libreoffice-collab/releases/latest)**.
2. Download **`instalar.bat`**.
3. **Double-click** it.
4. Wait. The app installs itself and opens when it is done.

> If Windows shows a blue warning ("Windows protected your PC"), click
> **More info** → **Run anyway**. This appears because the app is free and does
> not carry a paid Microsoft certificate.

### Option B — one line in PowerShell

```powershell
irm https://raw.githubusercontent.com/Cmprfda/libreoffice-collab/main/scripts/install.ps1 | iex
```

The installer downloads the latest release, installs into
`%LocalAppData%\Programs\LibreOffice Collab` (**no** administrator rights
needed), creates Desktop and Start Menu shortcuts, and launches the app.

---

## Step 2 — Set up the computer that shares the documents

Do this **once**, on **one** machine only — the one that will hold the files.
Pick one that is normally switched on.

1. Install **[Docker Desktop](https://www.docker.com/products/docker-desktop/)**
   and start it (wait for the icon to turn green).
2. Download **`instalar-servidor.bat`** from the
   [Releases](https://github.com/Cmprfda/libreoffice-collab/releases/latest) page.
3. Double-click it and accept the administrator prompt.

The script downloads the server, starts the Collabora engine, opens the required
firewall ports, creates the shared folder at
**`C:\Users\Public\Documentos Partilhados`**, and sets it all to start with
Windows.

**To share a document, copy it into that folder.** It appears in everyone's app
straight away.

---

## Step 3 — Day-to-day use

1. Open **LibreOffice Collab** from the Desktop shortcut.
2. The office server appears in the list on its own — no addresses or port
   numbers to type. The badge at the top shows **Connected**.
3. Click a document. An editor window opens.
4. Colleagues can open the same document at the same time. You see their named
   cursors and their changes as they type.
5. Saving is automatic.

**If the server does not appear:** go to **Settings → Server**, turn off
*Detect server automatically*, and type the address the server script printed
(for example `http://192.168.1.50:7373`). Click **Test connection**.

---

## Settings

| Setting | What it does |
| --- | --- |
| **Application language** | Português (PT) or English (EN). Applies instantly, no restart. |
| **Detect server automatically** | On by default. Finds the server by itself. |
| **Server address** | Manual fallback if discovery does not work. |
| **Theme** | System, Light or Dark. |
| **Start minimised** | Starts quietly in the notification area. |
| **Check for Updates** | Checks for a new version right now. |
| **Install updates automatically** | Installs new versions in the background. |

Updates are checked at startup and every 6 hours. Each one is signature-verified
before it is installed, so a tampered build is refused.

---

## FAQ

**Do documents leave the company?** No. Everything stays on your LAN. The only
internet connection is the update check against GitHub.

**Do I need LibreOffice installed?** No. Editing happens in the Collabora engine
on the host machine.

**What if the host is switched off?** The documents sit safely in the shared
folder. Everything reconnects by itself when it comes back.

**Supported formats:** `.odt`, `.ods`, `.odp`, `.odg`, `.docx`, `.xlsx`,
`.pptx`, `.doc`, `.xls`, `.ppt`, `.rtf`, `.csv`, `.txt`.

---
---

# 🔧 For developers

## Architecture

```
  Client (Tauri v2 · Rust + WebView2)          HOST MACHINE
  ┌─────────────────────────────┐        ┌──────────────────────────────┐
  │ React + TS dashboard        │        │  collab-server.exe (axum)    │
  │  · mDNS server list         │◄──────►│   · mDNS advert _locollab    │
  │  · document list            │  REST  │   · GET /api/documents       │
  │  · settings + i18n          │        │   · GET /api/session/{id}    │
  ├─────────────────────────────┤        │   · WOPI /wopi/files/{id}    │
  │ Editor window (WebView2)    │        └──────────────┬───────────────┘
  │  loads cool.html?WOPISrc=…  │◄──────────┐           │ WOPI
  └─────────────────────────────┘           │           ▼
                                            │  Collabora Online (Docker)
                                            └──────────── :9980
```

**Why Tauri v2 and not Electron:** WebView2 already ships with Windows 11, so the
installer is ~6 MB instead of ~120 MB and idle memory is roughly a third.
Mica, the tray and mDNS are all handled in Rust with no native Node modules.

**Why a server at all:** file-level locking is a property of the LibreOffice
desktop, not of the file system. The only way to get true co-authoring is to
have exactly one process own the document — that is what Collabora does, and
`collab-server` is the WOPI host that feeds it.

## Layout

| Path | What lives there |
| --- | --- |
| `src/` | React + TypeScript UI (dashboard, settings, i18n) |
| `src/i18n/pt.json` · `en.json` | Translation dictionaries — PT is the fallback |
| `src-tauri/src/` | Rust client: settings, mDNS browser, updater, tray, editor windows |
| `server/src/` | `collab-server`: REST API, WOPI host, mDNS advert |
| `docker/` | Collabora Online compose file |
| `scripts/` | Installers and the icon generator |
| `.github/workflows/` | CI and the tagged-release pipeline |

## Running locally

```bash
npm install
npm run icons          # generates src-tauri/icons/* (needs Rust for `tauri icon`)
npm run tauri:dev      # client, with hot reload

# in a second terminal — the host half
cd server
cargo run -- --dir ./documentos --port 7373 --cool http://localhost:9980

# and the engine
docker compose -f docker/docker-compose.yml up -d   # needs HOST_IP in docker/.env
```

## Before your first release

1. **Point the project at your repository.** Replace `Cmprfda/libreoffice-collab`
   in: `src/lib/config.ts`, `src-tauri/src/github.rs`, `src-tauri/tauri.conf.json`
   (updater endpoint), `scripts/instalar.bat`, `scripts/install.ps1`,
   `scripts/instalar-servidor.bat`, `scripts/host-setup.ps1`.

2. **Create the update signing key.**

   ```bash
   npm run tauri signer generate -- -w %USERPROFILE%\.tauri\collab.key
   ```

   - Put the **public** key in `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`.
   - Add the **private** key and its password as repository secrets named
     `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

3. **Release.**

   ```bash
   git tag v1.0.0 && git push origin v1.0.0
   ```

   The workflow syncs the version into every manifest, builds the NSIS
   installer, signs the update bundle, writes `latest.json` and attaches
   `collab-server.exe`, `instalar.bat` and `instalar-servidor.bat` to the
   release.

## Adding a translation string

1. Add the key to `src/i18n/pt.json` (the source of truth).
2. Add it to `src/i18n/en.json`. A missing key silently falls back to
   Portuguese, and `TranslationKey` is derived from `pt.json`, so TypeScript
   flags any key you use but never defined.
3. For strings Rust owns (tray menu, Windows toasts), edit
   `src-tauri/src/i18n.rs` as well.

## Security notes

- The server has **no authentication**: it is designed for a trusted office LAN
  and announces itself over mDNS. Put it behind a reverse proxy with auth before
  exposing it anywhere else.
- WOPI access tokens are per document, random, and expire after 12 hours. A
  token minted for one document cannot be used to read another.
- Document ids are hex-encoded file names and are validated on the way back, so
  a crafted id cannot escape the shared folder.
- Updates are verified against the minisign public key compiled into the app.

## Licence

MIT — see [LICENSE](LICENSE).
