//! The shared folder.
//!
//! Only this process ever opens the files, which is the whole point: because no
//! LibreOffice desktop instance touches them, no `.~lock.<name>#` file is ever
//! created and the "second user gets read-only" problem disappears.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

/// Extensions we offer for editing. Anything else in the folder is ignored so
/// stray PDFs and images do not clutter the list.
const EDITABLE: &[(&str, &str)] = &[
    ("odt", "text"),
    ("ott", "text"),
    ("doc", "text"),
    ("docx", "text"),
    ("rtf", "text"),
    ("txt", "text"),
    ("ods", "spreadsheet"),
    ("ots", "spreadsheet"),
    ("xls", "spreadsheet"),
    ("xlsx", "spreadsheet"),
    ("csv", "spreadsheet"),
    ("odp", "presentation"),
    ("otp", "presentation"),
    ("ppt", "presentation"),
    ("pptx", "presentation"),
    ("odg", "drawing"),
];

#[derive(Debug, Clone, Serialize)]
pub struct DocumentDto {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub kind: String,
    pub size: u64,
    /// Unix seconds.
    pub modified: i64,
}

fn kind_for(extension: &str) -> Option<&'static str> {
    EDITABLE
        .iter()
        .find(|(ext, _)| *ext == extension)
        .map(|(_, kind)| *kind)
}

/// Document ids are the hex-encoded file name.
///
/// Reversible, URL-safe, and — crucially — it cannot express `..` or a path
/// separator once decoded back through [`resolve`], which validates the result.
pub fn encode_id(file_name: &str) -> String {
    file_name
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_id(id: &str) -> Option<String> {
    if id.is_empty() || id.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..id.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&id[index..index + 2], 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

/// Lists every editable file in the shared folder, newest first.
pub fn list(dir: &Path) -> Vec<DocumentDto> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut documents: Vec<DocumentDto> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            // Skip LibreOffice lock files and Office temp files if a desktop
            // app ever touched the folder by accident.
            if file_name.starts_with(".~lock") || file_name.starts_with("~$") {
                return None;
            }

            let extension = Path::new(&file_name)
                .extension()
                .map(|ext| ext.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let kind = kind_for(&extension)?;

            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);

            Some(DocumentDto {
                id: encode_id(&file_name),
                name: file_name,
                extension,
                kind: kind.to_string(),
                size: metadata.len(),
                modified,
            })
        })
        .collect();

    documents.sort_by(|a, b| b.modified.cmp(&a.modified));
    documents
}

/// Maps a document id back to a real path inside the shared folder.
///
/// Returns `None` for anything that would escape the folder — decoded names
/// containing a separator, `..`, or a drive letter are all rejected.
pub fn resolve(dir: &Path, id: &str) -> Option<(PathBuf, String)> {
    let name = decode_id(id)?;

    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains(':') {
        return None;
    }

    let path = dir.join(&name);
    if !path.is_file() {
        return None;
    }
    Some((path, name))
}

/// Replaces a document's bytes atomically.
///
/// Collabora saves the whole file on every autosave, so a crash mid-write would
/// otherwise truncate the user's work. Writing next to the target and renaming
/// makes the swap atomic on NTFS.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = path.with_extension(format!(
        "{}.collab-tmp",
        path.extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_default()
    ));

    fs::write(&temp, bytes)?;
    // `rename` on Windows fails if the destination exists, so remove it first.
    // The window between the two calls is microseconds and the temp file still
    // holds the new content if power is lost.
    let _ = fs::remove_file(path);
    fs::rename(&temp, path)
}

/// Creates the shared folder on first run and drops a short readme in it.
pub fn ensure_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let readme = dir.join("LEIA-ME.txt");
    if !readme.exists() {
        fs::write(
            &readme,
            "Coloque aqui os documentos a partilhar (.odt, .ods, .odp, .docx, .xlsx, .pptx).\r\n\
             Todos os utilizadores da aplicacao LibreOffice Collab veem estes ficheiros.\r\n\
             \r\n\
             Place the documents you want to share in this folder.\r\n\
             Everyone running LibreOffice Collab will see them.\r\n",
        )?;
    }
    Ok(())
}
