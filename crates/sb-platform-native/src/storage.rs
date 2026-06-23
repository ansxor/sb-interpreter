//! Native filesystem [`Storage`](sb_core::storage::Storage) (M6-T1).
//!
//! Backs the device-neutral `sb-core` storage trait with a real directory tree laid out the
//! way SmileBASIC's on-device projects are — `<root>/<project>/{TXT,DAT}/<in-SB name>` — which
//! is exactly the shape the scraped corpus already unpacks to
//! (`harness/corpus/sbsave/files/<KEY>/{TXT,DAT}/`), so a corpus project drops straight in as
//! a ready LOAD/FILES fixture. Files hold the **logical body** (UTF-8 text / PCBN blob); the
//! extdata container wrap (header + HMAC footer) is applied only at the oracle boundary via
//! `sb_core::storage::extdata`, keeping this tree human-readable and corpus-compatible.
//!
//! This crate is the home for device I/O so `sb-core` stays wasm-safe. The whole module is
//! `cfg`-gated off `wasm32` (the wasm host uses IndexedDB instead, in `sb-platform-wasm`).

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sb_core::storage::{Folder, Storage, StorageError};

/// A [`Storage`] rooted at a host directory. Each project is a subdirectory containing `TXT/`
/// and `DAT/` folders; the `TXT`/`DAT` split mirrors SmileBASIC's on-device layout and the
/// `DAT:`/`TXT:` `FILES` filters.
#[derive(Debug, Clone)]
pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    /// Open (without creating) a store rooted at `root`. The directory is created lazily on
    /// the first `write`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        FsStorage { root: root.into() }
    }

    /// The directory holding one project's folder, e.g. `<root>/GAME/TXT`.
    fn folder_dir(&self, project: &str, folder: Folder) -> PathBuf {
        self.root.join(project).join(folder.dir_name())
    }

    /// The full path of one file's logical body.
    fn file_path(&self, project: &str, folder: Folder, name: &str) -> PathBuf {
        self.folder_dir(project, folder).join(name)
    }
}

/// Map a host I/O error to a [`StorageError`]: a missing path is `NotFound`, everything else
/// is a generic `Io` (both report errnum 46, Load failed).
fn io_err(e: io::Error) -> StorageError {
    if e.kind() == io::ErrorKind::NotFound {
        StorageError::NotFound
    } else {
        StorageError::Io(e.to_string())
    }
}

/// Sorted file names directly inside `dir`, or an empty list if `dir` does not exist.
fn list_dir(dir: &Path) -> Result<Vec<String>, StorageError> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(StorageError::Io(e.to_string())),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_err)?;
        if entry.file_type().map_err(io_err)?.is_file() {
            if let Some(n) = entry.file_name().to_str() {
                names.push(n.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

impl Storage for FsStorage {
    fn projects(&self) -> Result<Vec<String>, StorageError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(StorageError::Io(e.to_string())),
        };
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(io_err)?;
            if entry.file_type().map_err(io_err)?.is_dir() {
                if let Some(n) = entry.file_name().to_str() {
                    names.push(n.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    fn list(&self, project: &str, folder: Folder) -> Result<Vec<String>, StorageError> {
        list_dir(&self.folder_dir(project, folder))
    }

    fn exists(&self, project: &str, folder: Folder, name: &str) -> bool {
        self.file_path(project, folder, name).is_file()
    }

    fn read(&self, project: &str, folder: Folder, name: &str) -> Result<Vec<u8>, StorageError> {
        fs::read(self.file_path(project, folder, name)).map_err(io_err)
    }

    fn write(
        &mut self,
        project: &str,
        folder: Folder,
        name: &str,
        body: &[u8],
    ) -> Result<(), StorageError> {
        let dir = self.folder_dir(project, folder);
        fs::create_dir_all(&dir).map_err(io_err)?;
        fs::write(dir.join(name), body).map_err(io_err)
    }

    fn delete(&mut self, project: &str, folder: Folder, name: &str) -> Result<bool, StorageError> {
        match fs::remove_file(self.file_path(project, folder, name)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(StorageError::Io(e.to_string())),
        }
    }

    fn rename(
        &mut self,
        project: &str,
        folder: Folder,
        from: &str,
        to: &str,
    ) -> Result<(), StorageError> {
        let src = self.file_path(project, folder, from);
        if !src.is_file() {
            return Err(StorageError::NotFound);
        }
        let dst = self.file_path(project, folder, to);
        if from != to && dst.exists() {
            return Err(StorageError::AlreadyExists);
        }
        fs::rename(src, dst).map_err(io_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A scratch directory unique to this test run (no external temp-dir crate).
    fn scratch(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("sb-fsstorage-{tag}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn fs_write_read_roundtrip() {
        let dir = scratch("rt");
        let mut s = FsStorage::new(&dir);
        s.write("DEFAULT", Folder::Txt, "P", b"PRINT 1").unwrap();
        assert_eq!(
            s.read("DEFAULT", Folder::Txt, "P").unwrap(),
            b"PRINT 1".to_vec()
        );
        assert!(s.exists("DEFAULT", Folder::Txt, "P"));
        // The on-disk layout matches SmileBASIC's PROJECTS/<name>/{TXT,DAT}/ tree.
        assert!(dir.join("DEFAULT").join("TXT").join("P").is_file());
        assert_eq!(s.projects().unwrap(), vec!["DEFAULT".to_string()]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fs_missing_is_not_found() {
        let dir = scratch("missing");
        let s = FsStorage::new(&dir);
        assert_eq!(
            s.read("DEFAULT", Folder::Txt, "NOPE"),
            Err(StorageError::NotFound)
        );
        assert!(!s.exists("DEFAULT", Folder::Txt, "NOPE"));
        // Listing a missing project/folder is empty, not an error.
        assert_eq!(s.list("GHOST", Folder::Dat).unwrap(), Vec::<String>::new());
        assert_eq!(s.projects().unwrap(), Vec::<String>::new());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fs_list_delete_rename() {
        let dir = scratch("ops");
        let mut s = FsStorage::new(&dir);
        for n in ["GAMMA", "ALPHA", "BETA"] {
            s.write("P", Folder::Txt, n, b"x").unwrap();
        }
        assert_eq!(
            s.list("P", Folder::Txt).unwrap(),
            vec!["ALPHA".to_string(), "BETA".to_string(), "GAMMA".to_string()]
        );

        assert!(s.delete("P", Folder::Txt, "BETA").unwrap());
        assert!(!s.delete("P", Folder::Txt, "BETA").unwrap());

        s.rename("P", Folder::Txt, "ALPHA", "OMEGA").unwrap();
        assert!(!s.exists("P", Folder::Txt, "ALPHA"));
        assert_eq!(s.read("P", Folder::Txt, "OMEGA").unwrap(), b"x".to_vec());

        assert_eq!(
            s.rename("P", Folder::Txt, "GHOST", "X"),
            Err(StorageError::NotFound)
        );
        assert_eq!(
            s.rename("P", Folder::Txt, "GAMMA", "OMEGA"),
            Err(StorageError::AlreadyExists)
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fs_extdata_export_import() {
        // The native body is the logical payload; wrapping it through the extdata codec yields
        // bytes real SB accepts, and unwrapping recovers the body — the oracle interop path.
        use sb_core::storage::extdata::{unwrap, wrap, Marker};
        let dir = scratch("extdata");
        let mut s = FsStorage::new(&dir);
        s.write("DEFAULT", Folder::Txt, "P", b"PRINT 1").unwrap();
        let body = s.read("DEFAULT", Folder::Txt, "P").unwrap();
        let container = wrap(Marker::Txt, &body);
        let (marker, back) = unwrap(&container).unwrap();
        assert_eq!(marker, Marker::Txt);
        assert_eq!(back, body);
        fs::remove_dir_all(&dir).ok();
    }
}
