//! Storage abstraction (M6-T1) — the device-neutral file/project layer the M6 file
//! commands (SAVE/LOAD/FILES/DELETE/RENAME/CHKFILE/PROJECT) sit on top of.
//!
//! SmileBASIC addresses every file by a string `"[ResourceType:]Name"` inside a *project*
//! folder; on the 3DS those folders mirror `PROJECTS/<name>/{TXT,DAT}/` and each file is an
//! 80-byte-header + body + 20-byte-HMAC-footer **extdata container** (see
//! `spec/concepts/file-and-extdata-format.md`). This module owns the I/O-free half of that
//! model so it stays wasm-safe and exercised by the deterministic gate:
//!
//! * the **logical resource model** — [`ResourceKind`]/[`parse_resource`] split a
//!   `"TYPE:NAME"` string into a typed namespace + name, with the disassembled errnum
//!   mapping (unknown type → 4, index past its family → 10), and [`FilesFilter`] for `FILES`;
//! * the [`Storage`] trait — `projects`/`list`/`read`/`write`/`delete`/`rename`/`exists`
//!   keyed by `(project, folder, in-SB name)`, implemented by the platform crates
//!   (native filesystem, wasm IndexedDB) and by the in-memory [`MemStorage`] used in tests;
//! * the [`extdata`] codec — pure `wrap`/`unwrap` of the on-disk container (header markers +
//!   HMAC-SHA1 footer) so the layer can import/export real-SmileBASIC files for the oracle
//!   (O-T3). The HMAC key, markers and footer are `hw_verified` round-trip against SB 3.6.0
//!   (`sb-oracle` `sb_extdata.py`); this is the same byte layout, reimplemented dependency-free.
//!
//! Actual device I/O lives in the `sb-platform-*` crates; `sb-core` only defines the trait,
//! the pure codec and the test impl. Names at the trait boundary are **in-SB names** (no
//! `T`/`B` on-disk prefix) — the [`Folder`] already encodes the type, and the prefix is an
//! on-disk detail platform impls add when they target the real extdata directory.

use std::collections::BTreeMap;

/// The default project SmileBASIC starts in (`PROJECT ""` resets to this). On the device,
/// files with no other project selected live under `PROJECTS/DEFAULT/`.
pub const DEFAULT_PROJECT: &str = "DEFAULT";

/// The two on-disk folders a project splits into. SmileBASIC keeps text/programs in `TXT/`
/// and binary/graphics in `DAT/`; the `T`/`B` on-disk name prefixes and the `TXT:`/`DAT:`
/// `FILES` filters both encode exactly this split (see the format concept §2/§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Folder {
    /// `TXT/` — UTF-8 program source and `TXT:` string resources. On-disk prefix `T`.
    Txt,
    /// `DAT/` — PCBN binary: `DAT:` numeric arrays and the `GRP*` graphic pages. Prefix `B`.
    Dat,
}

impl Folder {
    /// The single-character on-disk name prefix (`TXT → 'T'`, `DAT → 'B'`). The on-disk
    /// filename is this prefix followed by the in-SB name (concept §2).
    pub fn prefix(self) -> char {
        match self {
            Folder::Txt => 'T',
            Folder::Dat => 'B',
        }
    }

    /// The `PROJECTS/<project>/<DIR>/` subdirectory name (`"TXT"` / `"DAT"`).
    pub fn dir_name(self) -> &'static str {
        match self {
            Folder::Txt => "TXT",
            Folder::Dat => "DAT",
        }
    }
}

/// A logical typed namespace from a `"TYPE:NAME"` resource string (concept §1). Several
/// kinds share a [`Folder`]: programs are stored as `TXT` files, and the `GRP*` pages share
/// the `DAT` folder with `DAT:` arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// `PRG0:`–`PRG3:` (and bare `PRG:` == `PRG0:`) — a program slot's source. Stored as a
    /// `TXT` file: there is no separate program container on disk.
    Program(u8),
    /// `GRP0:`–`GRP5:` — a 512×512 graphic page.
    Graphic(u8),
    /// `GRPF:` — the font/sprite-sheet page (page index −1 in GSAVE).
    GraphicFont,
    /// `TXT:` — a UTF-8 string resource.
    Text,
    /// `DAT:` — a PCBN numeric-array resource.
    Data,
}

impl ResourceKind {
    /// Which on-disk folder this namespace is stored in.
    pub fn folder(self) -> Folder {
        match self {
            ResourceKind::Program(_) | ResourceKind::Text => Folder::Txt,
            ResourceKind::Graphic(_) | ResourceKind::GraphicFont | ResourceKind::Data => {
                Folder::Dat
            }
        }
    }
}

/// What a `"[TYPE:]NAME"` string resolved to: an explicit [`ResourceKind`], or `Bare` when
/// there was no `TYPE:` prefix. A bare name defaults to the running program slot for
/// `SAVE`/`LOAD` and to the `TXT` namespace for `CHKFILE` — the caller picks, since only it
/// knows the current slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceSpec {
    /// No `TYPE:` prefix — the caller resolves the default namespace.
    Bare,
    /// An explicit typed namespace.
    Kind(ResourceKind),
}

/// Why a resource string failed to parse, with the errnum SmileBASIC raises for it. From the
/// `SAVE` handler `@0x18e7d4`: an unrecognized resource type takes `mov r0,#0x4` (errnum 4),
/// an index past its family's range takes `mov r0,#0xa` (errnum 10) — see the format concept §1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceError {
    /// Unrecognized `TYPE:` prefix → **errnum 4** (Illegal function call).
    UnknownType,
    /// A valid family but the index is past its limit (e.g. `PRG4`, `GRP6`) → **errnum 10**.
    IndexOutOfRange,
}

impl ResourceError {
    /// The SmileBASIC error number for this failure.
    pub fn errnum(self) -> u8 {
        match self {
            ResourceError::UnknownType => 4,
            ResourceError::IndexOutOfRange => 10,
        }
    }
}

/// Split a `"[TYPE:]NAME"` resource string into its typed namespace and the bare name.
///
/// With no `:` the whole string is the name and the spec is [`ResourceSpec::Bare`]. With a
/// prefix, the type is matched case-insensitively (SmileBASIC keywords are case-folded): the
/// `PRG`/`GRP` families take an optional single-digit index (`PRG` == `PRG0`, `GRP` == `GRP0`),
/// `GRPF` is the font page, and `TXT`/`DAT` are the string/array resources. The errnum
/// mapping follows the disassembled `SAVE` handler (unknown type → 4, index past range → 10).
pub fn parse_resource(s: &str) -> Result<(ResourceSpec, &str), ResourceError> {
    let Some(colon) = s.find(':') else {
        return Ok((ResourceSpec::Bare, s));
    };
    let (ty, rest) = s.split_at(colon);
    let name = &rest[1..]; // drop the ':'
    let upper = ty.to_ascii_uppercase();

    let kind = match upper.as_str() {
        "TXT" => ResourceKind::Text,
        "DAT" => ResourceKind::Data,
        "GRPF" => ResourceKind::GraphicFont,
        _ => {
            if let Some(idx) = upper.strip_prefix("PRG") {
                ResourceKind::Program(parse_index(idx, 3)?)
            } else if let Some(idx) = upper.strip_prefix("GRP") {
                ResourceKind::Graphic(parse_index(idx, 5)?)
            } else {
                return Err(ResourceError::UnknownType);
            }
        }
    };
    Ok((ResourceSpec::Kind(kind), name))
}

/// Parse the optional single-digit index after a `PRG`/`GRP` prefix, bounded by `max`
/// (inclusive). Empty → 0 (bare `PRG`/`GRP`). A non-digit suffix is an unknown type
/// (errnum 4); a digit past `max` is out of range (errnum 10).
fn parse_index(idx: &str, max: u8) -> Result<u8, ResourceError> {
    if idx.is_empty() {
        return Ok(0);
    }
    let n: u8 = idx.parse().map_err(|_| ResourceError::UnknownType)?;
    if n > max {
        return Err(ResourceError::IndexOutOfRange);
    }
    Ok(n)
}

/// A `FILES [filter]` selector (concept §1). Decides what `FILES` lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesFilter {
    /// No filter — all files in the current project.
    All,
    /// `"TXT:"` — texts **and** programs (the `TXT` folder).
    Txt,
    /// `"DAT:"` — binary data **including** graphics (the `DAT` folder).
    Dat,
    /// `"//"` — the project list (all projects).
    Projects,
    /// `"NAME/"` — the contents of the named project.
    Project(String),
}

/// Parse a `FILES` filter string. `"//"` is the project list, a trailing `/` names a project,
/// `"TXT:"`/`"DAT:"` (case-insensitively) select a folder, and anything else lists everything.
pub fn parse_files_filter(s: &str) -> FilesFilter {
    if s.is_empty() {
        return FilesFilter::All;
    }
    if s == "//" {
        return FilesFilter::Projects;
    }
    if let Some(proj) = s.strip_suffix('/') {
        return FilesFilter::Project(proj.to_string());
    }
    match s.to_ascii_uppercase().as_str() {
        "TXT:" | "TXT" => FilesFilter::Txt,
        "DAT:" | "DAT" => FilesFilter::Dat,
        _ => FilesFilter::All,
    }
}

/// A storage operation failure, with the SmileBASIC errnum a file command raises for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// The file does not exist → **errnum 46** (Load failed).
    NotFound,
    /// The bytes are not a format SmileBASIC can read (bad container / wrong footer) →
    /// **errnum 35** (Illegal file format).
    IllegalFormat,
    /// A rename target name already exists.
    AlreadyExists,
    /// A host I/O failure (permission, disk, IndexedDB) → **errnum 46** (Load failed).
    Io(String),
}

impl StorageError {
    /// The SmileBASIC error number a file command reports for this failure.
    pub fn errnum(&self) -> u8 {
        match self {
            StorageError::IllegalFormat => 35,
            StorageError::NotFound | StorageError::AlreadyExists | StorageError::Io(_) => 46,
        }
    }
}

impl core::fmt::Display for StorageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StorageError::NotFound => write!(f, "file not found"),
            StorageError::IllegalFormat => write!(f, "illegal file format"),
            StorageError::AlreadyExists => write!(f, "file already exists"),
            StorageError::Io(e) => write!(f, "i/o error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

/// The device-neutral file/project layer the M6 file commands sit on.
///
/// Every access is keyed by `(project, folder, in-SB name)`. Bodies are the **logical
/// resource payload** (UTF-8 text or PCBN blob) — wrapping into / unwrapping out of the
/// on-disk extdata container ([`extdata`]) is a platform-boundary concern, not part of the
/// trait, so the in-memory and corpus-tree representations stay plain. Implementations must
/// auto-create a project on first `write`, and `list`/`projects` must return **sorted**
/// names so the gate is deterministic.
pub trait Storage {
    /// All project names, sorted.
    fn projects(&self) -> Result<Vec<String>, StorageError>;

    /// The in-SB names in one folder of a project, sorted. A missing project/folder is an
    /// empty list, not an error.
    fn list(&self, project: &str, folder: Folder) -> Result<Vec<String>, StorageError>;

    /// Whether a file exists. Never errors (a missing project is just `false`).
    fn exists(&self, project: &str, folder: Folder, name: &str) -> bool;

    /// Read a file's logical body. [`StorageError::NotFound`] if it does not exist.
    fn read(&self, project: &str, folder: Folder, name: &str) -> Result<Vec<u8>, StorageError>;

    /// Write (creating or overwriting) a file's logical body, creating the project if needed.
    fn write(
        &mut self,
        project: &str,
        folder: Folder,
        name: &str,
        body: &[u8],
    ) -> Result<(), StorageError>;

    /// Delete a file. Returns `true` if it existed, `false` if there was nothing to delete.
    fn delete(&mut self, project: &str, folder: Folder, name: &str) -> Result<bool, StorageError>;

    /// Rename `from` → `to` in one folder. [`StorageError::NotFound`] if `from` is missing,
    /// [`StorageError::AlreadyExists`] if `to` already exists.
    fn rename(
        &mut self,
        project: &str,
        folder: Folder,
        from: &str,
        to: &str,
    ) -> Result<(), StorageError>;
}

/// One project's two folders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProjectFiles {
    txt: BTreeMap<String, Vec<u8>>,
    dat: BTreeMap<String, Vec<u8>>,
}

impl ProjectFiles {
    fn folder(&self, f: Folder) -> &BTreeMap<String, Vec<u8>> {
        match f {
            Folder::Txt => &self.txt,
            Folder::Dat => &self.dat,
        }
    }
    fn folder_mut(&mut self, f: Folder) -> &mut BTreeMap<String, Vec<u8>> {
        match f {
            Folder::Txt => &mut self.txt,
            Folder::Dat => &mut self.dat,
        }
    }
}

/// An in-memory [`Storage`] for tests and the wasm IndexedDB mirror. Deterministic
/// (`BTreeMap`-backed, sorted listings) so it can seed the `sbsave` corpus tree as a ready
/// project and serve the conformance gate without any host I/O.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemStorage {
    projects: BTreeMap<String, ProjectFiles>,
}

impl MemStorage {
    /// An empty store (no projects yet — the first `write` creates one).
    pub fn new() -> Self {
        MemStorage::default()
    }

    /// Serialize the whole store to a deterministic byte blob (length-prefixed, sorted by the
    /// `BTreeMap` order). This is the wasm host's IndexedDB payload: the browser layer shuttles
    /// one opaque blob, while the (de)serialization stays here in the wasm-safe, gate-tested
    /// core. `serialize` → `deserialize` round-trips, and equal stores serialize equal.
    pub fn serialize(&self) -> Vec<u8> {
        fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        }
        let mut out = Vec::new();
        out.extend_from_slice(&(self.projects.len() as u32).to_le_bytes());
        for (name, files) in &self.projects {
            put_bytes(&mut out, name.as_bytes());
            for folder in [&files.txt, &files.dat] {
                out.extend_from_slice(&(folder.len() as u32).to_le_bytes());
                for (fname, body) in folder {
                    put_bytes(&mut out, fname.as_bytes());
                    put_bytes(&mut out, body);
                }
            }
        }
        out
    }

    /// Rebuild a store from [`serialize`](Self::serialize)'s blob. A truncated or malformed
    /// blob is [`StorageError::IllegalFormat`].
    pub fn deserialize(data: &[u8]) -> Result<MemStorage, StorageError> {
        struct Reader<'a> {
            data: &'a [u8],
            pos: usize,
        }
        impl Reader<'_> {
            fn u32(&mut self) -> Result<usize, StorageError> {
                let end = self.pos + 4;
                let slice = self
                    .data
                    .get(self.pos..end)
                    .ok_or(StorageError::IllegalFormat)?;
                self.pos = end;
                Ok(u32::from_le_bytes(slice.try_into().expect("4 bytes")) as usize)
            }
            fn bytes(&mut self) -> Result<Vec<u8>, StorageError> {
                let len = self.u32()?;
                let end = self.pos + len;
                let slice = self
                    .data
                    .get(self.pos..end)
                    .ok_or(StorageError::IllegalFormat)?;
                self.pos = end;
                Ok(slice.to_vec())
            }
            fn string(&mut self) -> Result<String, StorageError> {
                String::from_utf8(self.bytes()?).map_err(|_| StorageError::IllegalFormat)
            }
        }

        let mut r = Reader { data, pos: 0 };
        let mut projects = BTreeMap::new();
        let nprojects = r.u32()?;
        for _ in 0..nprojects {
            let pname = r.string()?;
            let mut files = ProjectFiles::default();
            for folder in [Folder::Txt, Folder::Dat] {
                let nfiles = r.u32()?;
                for _ in 0..nfiles {
                    let fname = r.string()?;
                    let body = r.bytes()?;
                    files.folder_mut(folder).insert(fname, body);
                }
            }
            projects.insert(pname, files);
        }
        if r.pos != data.len() {
            return Err(StorageError::IllegalFormat);
        }
        Ok(MemStorage { projects })
    }
}

impl Storage for MemStorage {
    fn projects(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.projects.keys().cloned().collect())
    }

    fn list(&self, project: &str, folder: Folder) -> Result<Vec<String>, StorageError> {
        Ok(self
            .projects
            .get(project)
            .map(|p| p.folder(folder).keys().cloned().collect())
            .unwrap_or_default())
    }

    fn exists(&self, project: &str, folder: Folder, name: &str) -> bool {
        self.projects
            .get(project)
            .is_some_and(|p| p.folder(folder).contains_key(name))
    }

    fn read(&self, project: &str, folder: Folder, name: &str) -> Result<Vec<u8>, StorageError> {
        self.projects
            .get(project)
            .and_then(|p| p.folder(folder).get(name))
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    fn write(
        &mut self,
        project: &str,
        folder: Folder,
        name: &str,
        body: &[u8],
    ) -> Result<(), StorageError> {
        self.projects
            .entry(project.to_string())
            .or_default()
            .folder_mut(folder)
            .insert(name.to_string(), body.to_vec());
        Ok(())
    }

    fn delete(&mut self, project: &str, folder: Folder, name: &str) -> Result<bool, StorageError> {
        Ok(self
            .projects
            .get_mut(project)
            .is_some_and(|p| p.folder_mut(folder).remove(name).is_some()))
    }

    fn rename(
        &mut self,
        project: &str,
        folder: Folder,
        from: &str,
        to: &str,
    ) -> Result<(), StorageError> {
        let files = self
            .projects
            .get_mut(project)
            .map(|p| p.folder_mut(folder))
            .ok_or(StorageError::NotFound)?;
        if !files.contains_key(from) {
            return Err(StorageError::NotFound);
        }
        if from != to && files.contains_key(to) {
            return Err(StorageError::AlreadyExists);
        }
        let body = files.remove(from).unwrap();
        files.insert(to.to_string(), body);
        Ok(())
    }
}

/// The on-disk extdata container codec — pure `wrap`/`unwrap` of one SmileBASIC file
/// (`80-byte header || body || 20-byte HMAC-SHA1 footer`). This is the bridge between the
/// [`Storage`] trait's logical bodies and the bytes real SB 3.6.0 reads/writes, so a platform
/// impl targeting the actual extdata directory (or the oracle, O-T3) can import/export real
/// files. The format is `hw_verified` round-trip against SB 3.6.0 (`sb-oracle sb_extdata.py`);
/// the HMAC-SHA1 here is a dependency-free reimplementation, cross-checked byte-for-byte
/// against that tool in the unit tests.
pub mod extdata {
    use super::{Folder, StorageError};

    /// SmileBASIC's file-integrity HMAC-SHA1 key (from `nnn1590/lpp-3ds-sbfm`). The footer is
    /// an integrity check, not encryption — the body is plaintext; a wrong footer makes SB
    /// refuse the file (`?NAME` in `FILES`).
    pub const HMAC_KEY: &[u8] =
        b"nqmby+e9S?{%U*-V]51n%^xZMk8>b{?x]&?(NmmV[,g85:%6Sqd\"'U\")/8u77UL2";

    /// Header length (0x50).
    pub const HEADER_LEN: usize = 80;
    /// Footer length (0x14) — the HMAC-SHA1 digest.
    pub const FOOTER_LEN: usize = 20;

    /// The fixed save-date bytes SBFM/the injector stamps at header offset 0x0C. SB itself
    /// writes the real RTC here; the value is not validated on load (see concept §2).
    const DATE: [u8; 4] = [0xDF, 0x07, 0x0A, 0x0F];

    /// Which container type marker a body carries. `Grp` and `Dat` both live in the `DAT`
    /// folder but stamp distinct 8-byte markers, so the codec is keyed on the marker, not
    /// just the [`Folder`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Marker {
        /// UTF-8 text/program (on-disk prefix `T`, folder `TXT`).
        Txt,
        /// PCBN numeric-array data (prefix `B`, folder `DAT`).
        Dat,
        /// PCBN graphic page (prefix `B`, folder `DAT`).
        Grp,
    }

    impl Marker {
        /// The 8-byte type marker at header offset 0x00 (concept §2).
        fn bytes(self) -> [u8; 8] {
            match self {
                Marker::Txt => [0x01, 0, 0, 0, 0, 0, 0x01, 0],
                Marker::Dat => [0x01, 0, 0x01, 0, 0, 0, 0, 0],
                Marker::Grp => [0x01, 0, 0x01, 0, 0, 0, 0x02, 0],
            }
        }

        fn from_bytes(b: &[u8]) -> Option<Marker> {
            [Marker::Txt, Marker::Dat, Marker::Grp]
                .into_iter()
                .find(|m| b == m.bytes())
        }

        /// The on-disk folder this marker's files live in.
        pub fn folder(self) -> Folder {
            match self {
                Marker::Txt => Folder::Txt,
                Marker::Dat | Marker::Grp => Folder::Dat,
            }
        }
    }

    /// Build the exact on-disk container bytes for a logical body: `header || body || footer`
    /// with a valid HMAC-SHA1 footer SB 3.6.0 accepts.
    pub fn wrap(marker: Marker, body: &[u8]) -> Vec<u8> {
        let mut header = Vec::with_capacity(HEADER_LEN);
        header.extend_from_slice(&marker.bytes());
        header.extend_from_slice(&(body.len() as u32).to_le_bytes());
        header.extend_from_slice(&DATE);
        header.resize(HEADER_LEN, 0);

        let mut out = Vec::with_capacity(HEADER_LEN + body.len() + FOOTER_LEN);
        out.extend_from_slice(&header);
        out.extend_from_slice(body);
        let footer = hmac_sha1(HMAC_KEY, &out);
        out.extend_from_slice(&footer);
        out
    }

    /// Parse an on-disk container back into `(marker, logical body)`. Rejects a too-short
    /// blob, an unknown type marker, a body-length field that disagrees with the data, or a
    /// footer that fails the HMAC check ([`StorageError::IllegalFormat`]).
    pub fn unwrap(data: &[u8]) -> Result<(Marker, Vec<u8>), StorageError> {
        if data.len() < HEADER_LEN + FOOTER_LEN {
            return Err(StorageError::IllegalFormat);
        }
        let marker = Marker::from_bytes(&data[0..8]).ok_or(StorageError::IllegalFormat)?;
        let body_len = u32::from_le_bytes(data[8..12].try_into().expect("4 bytes")) as usize;
        let body_end = HEADER_LEN
            .checked_add(body_len)
            .ok_or(StorageError::IllegalFormat)?;
        if data.len() != body_end + FOOTER_LEN {
            return Err(StorageError::IllegalFormat);
        }
        let expected = hmac_sha1(HMAC_KEY, &data[..body_end]);
        if expected != data[body_end..] {
            return Err(StorageError::IllegalFormat);
        }
        Ok((marker, data[HEADER_LEN..body_end].to_vec()))
    }

    /// HMAC-SHA1 (RFC 2104) over `msg` with `key`, dependency-free.
    pub fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
        const BLOCK: usize = 64;
        let mut k = [0u8; BLOCK];
        if key.len() > BLOCK {
            k[..20].copy_from_slice(&sha1(key));
        } else {
            k[..key.len()].copy_from_slice(key);
        }
        let mut ipad = [0x36u8; BLOCK];
        let mut opad = [0x5cu8; BLOCK];
        for i in 0..BLOCK {
            ipad[i] ^= k[i];
            opad[i] ^= k[i];
        }
        let mut inner = Vec::with_capacity(BLOCK + msg.len());
        inner.extend_from_slice(&ipad);
        inner.extend_from_slice(msg);
        let inner_hash = sha1(&inner);
        let mut outer = Vec::with_capacity(BLOCK + 20);
        outer.extend_from_slice(&opad);
        outer.extend_from_slice(&inner_hash);
        sha1(&outer)
    }

    /// SHA-1 (FIPS 180-1) of `msg`, dependency-free.
    pub fn sha1(msg: &[u8]) -> [u8; 20] {
        let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

        // Pad: 0x80, then zeros to 56 mod 64, then the 64-bit big-endian bit length.
        let mut data = msg.to_vec();
        let bit_len = (msg.len() as u64).wrapping_mul(8);
        data.push(0x80);
        while data.len() % 64 != 56 {
            data.push(0);
        }
        data.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in data.chunks_exact(64) {
            let mut w = [0u32; 80];
            for (i, word) in chunk.chunks_exact(4).enumerate() {
                w[i] = u32::from_be_bytes(word.try_into().expect("4 bytes"));
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let [mut a, mut b, mut c, mut d, mut e] = h;
            for (i, &wi) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | (!b & d), 0x5A827999),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                    _ => (b ^ c ^ d, 0xCA62C1D6),
                };
                let tmp = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(wi);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = tmp;
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
        }

        let mut out = [0u8; 20];
        for (i, word) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::extdata::{hmac_sha1, sha1, unwrap, wrap, Marker, HEADER_LEN, HMAC_KEY};
    use super::*;

    // --- resource-string parsing (concept §1) ---

    #[test]
    fn parses_typed_resources() {
        assert_eq!(
            parse_resource("TXT:FOO"),
            Ok((ResourceSpec::Kind(ResourceKind::Text), "FOO"))
        );
        assert_eq!(
            parse_resource("DAT:A"),
            Ok((ResourceSpec::Kind(ResourceKind::Data), "A"))
        );
        assert_eq!(
            parse_resource("PRG2:GAME"),
            Ok((ResourceSpec::Kind(ResourceKind::Program(2)), "GAME"))
        );
        assert_eq!(
            parse_resource("GRP0:PIC"),
            Ok((ResourceSpec::Kind(ResourceKind::Graphic(0)), "PIC"))
        );
        assert_eq!(
            parse_resource("GRPF:FONT"),
            Ok((ResourceSpec::Kind(ResourceKind::GraphicFont), "FONT"))
        );
    }

    #[test]
    fn bare_prefix_is_index_zero_and_case_folds() {
        // PRG == PRG0, GRP == GRP0; type is matched case-insensitively.
        assert_eq!(
            parse_resource("PRG:X"),
            Ok((ResourceSpec::Kind(ResourceKind::Program(0)), "X"))
        );
        assert_eq!(
            parse_resource("grp:Y"),
            Ok((ResourceSpec::Kind(ResourceKind::Graphic(0)), "Y"))
        );
    }

    #[test]
    fn no_prefix_is_bare() {
        assert_eq!(parse_resource("TEST"), Ok((ResourceSpec::Bare, "TEST")));
    }

    #[test]
    fn resource_errnum_mapping() {
        // Unknown type -> errnum 4 (Illegal function call).
        assert_eq!(parse_resource("FOO:X"), Err(ResourceError::UnknownType));
        assert_eq!(ResourceError::UnknownType.errnum(), 4);
        // Index past family range -> errnum 10 (Out of range): PRG4, GRP6.
        assert_eq!(
            parse_resource("PRG4:X"),
            Err(ResourceError::IndexOutOfRange)
        );
        assert_eq!(
            parse_resource("GRP6:X"),
            Err(ResourceError::IndexOutOfRange)
        );
        assert_eq!(ResourceError::IndexOutOfRange.errnum(), 10);
    }

    #[test]
    fn resource_kind_folder_mapping() {
        // Programs are TXT files; GRP pages share the DAT folder.
        assert_eq!(ResourceKind::Program(1).folder(), Folder::Txt);
        assert_eq!(ResourceKind::Text.folder(), Folder::Txt);
        assert_eq!(ResourceKind::Graphic(3).folder(), Folder::Dat);
        assert_eq!(ResourceKind::GraphicFont.folder(), Folder::Dat);
        assert_eq!(ResourceKind::Data.folder(), Folder::Dat);
    }

    #[test]
    fn files_filter_parsing() {
        assert_eq!(parse_files_filter(""), FilesFilter::All);
        assert_eq!(parse_files_filter("TXT:"), FilesFilter::Txt);
        assert_eq!(parse_files_filter("DAT:"), FilesFilter::Dat);
        assert_eq!(parse_files_filter("//"), FilesFilter::Projects);
        assert_eq!(
            parse_files_filter("GAME/"),
            FilesFilter::Project("GAME".to_string())
        );
    }

    // --- in-memory Storage round-trips ---

    #[test]
    fn mem_storage_write_read_roundtrip() {
        let mut s = MemStorage::new();
        s.write(DEFAULT_PROJECT, Folder::Txt, "P", b"PRINT 1")
            .unwrap();
        assert_eq!(
            s.read(DEFAULT_PROJECT, Folder::Txt, "P").unwrap(),
            b"PRINT 1"
        );
        assert!(s.exists(DEFAULT_PROJECT, Folder::Txt, "P"));
        // First write created the project.
        assert_eq!(s.projects().unwrap(), vec![DEFAULT_PROJECT.to_string()]);
    }

    #[test]
    fn mem_storage_missing_is_not_found() {
        let s = MemStorage::new();
        assert_eq!(
            s.read("DEFAULT", Folder::Txt, "NOPE"),
            Err(StorageError::NotFound)
        );
        assert!(!s.exists("DEFAULT", Folder::Txt, "NOPE"));
        assert_eq!(StorageError::NotFound.errnum(), 46);
    }

    #[test]
    fn mem_storage_folders_are_independent() {
        let mut s = MemStorage::new();
        s.write("P", Folder::Txt, "A", b"text").unwrap();
        s.write("P", Folder::Dat, "A", b"data").unwrap();
        // Same name in both folders, distinct bodies.
        assert_eq!(s.read("P", Folder::Txt, "A").unwrap(), b"text");
        assert_eq!(s.read("P", Folder::Dat, "A").unwrap(), b"data");
        assert_eq!(s.list("P", Folder::Txt).unwrap(), vec!["A".to_string()]);
    }

    #[test]
    fn mem_storage_list_is_sorted() {
        let mut s = MemStorage::new();
        for n in ["GAMMA", "ALPHA", "BETA"] {
            s.write("P", Folder::Txt, n, b"x").unwrap();
        }
        assert_eq!(
            s.list("P", Folder::Txt).unwrap(),
            vec!["ALPHA".to_string(), "BETA".to_string(), "GAMMA".to_string()]
        );
        assert_eq!(
            s.list("MISSING", Folder::Txt).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn mem_storage_delete_and_rename() {
        let mut s = MemStorage::new();
        s.write("P", Folder::Txt, "OLD", b"x").unwrap();
        // delete reports whether it existed.
        assert!(s.delete("P", Folder::Txt, "OLD").unwrap());
        assert!(!s.delete("P", Folder::Txt, "OLD").unwrap());

        s.write("P", Folder::Txt, "OLD", b"y").unwrap();
        s.rename("P", Folder::Txt, "OLD", "NEW").unwrap();
        assert!(!s.exists("P", Folder::Txt, "OLD"));
        assert_eq!(s.read("P", Folder::Txt, "NEW").unwrap(), b"y");

        // rename of a missing source / onto an existing target.
        assert_eq!(
            s.rename("P", Folder::Txt, "GHOST", "X"),
            Err(StorageError::NotFound)
        );
        s.write("P", Folder::Txt, "OTHER", b"z").unwrap();
        assert_eq!(
            s.rename("P", Folder::Txt, "NEW", "OTHER"),
            Err(StorageError::AlreadyExists)
        );
    }

    // --- extdata container codec ---

    #[test]
    fn sha1_known_vector() {
        // FIPS 180-1 / well-known: SHA1("abc").
        assert_eq!(
            sha1(b"abc"),
            hex("a9993e364706816aba3e25717850c26c9cd0d89d")
        );
        // Empty string.
        assert_eq!(sha1(b""), hex("da39a3ee5e6b4b0d3255bfef95601890afd80709"));
    }

    #[test]
    fn hmac_sha1_rfc2202_vector() {
        // RFC 2202 test case 1: key = 0x0b*20, data = "Hi There".
        assert_eq!(
            hmac_sha1(&[0x0b; 20], b"Hi There"),
            hex("b617318655057264e28bc0b6fb378c8ef146be00")
        );
    }

    #[test]
    fn extdata_footer_matches_real_sb() {
        // Golden from the oracle's own sb_extdata.py (hw_verified key + format): the footer of
        // a TXT container wrapping b"PRINT 1". Proves our dependency-free HMAC-SHA1 + header
        // layout are byte-identical to what SB 3.6.0 accepts.
        let container = wrap(Marker::Txt, b"PRINT 1");
        let footer = &container[container.len() - 20..];
        assert_eq!(footer, hex("6d7b94ed26ffda88dc7f968437e5a53f08990cb2"));
        // Header: marker, then LE body length, then the fixed date.
        assert_eq!(&container[0..8], &[0x01, 0, 0, 0, 0, 0, 0x01, 0]);
        assert_eq!(&container[8..12], &7u32.to_le_bytes());
        assert_eq!(container.len(), HEADER_LEN + 7 + 20);
    }

    #[test]
    fn extdata_wrap_unwrap_roundtrip() {
        for (marker, body) in [
            (Marker::Txt, b"PRINT \"HELLO\"".to_vec()),
            (Marker::Dat, vec![1, 2, 3, 4, 5]),
            (Marker::Grp, vec![0u8; 100]),
        ] {
            let container = wrap(marker, &body);
            let (got_marker, got_body) = unwrap(&container).unwrap();
            assert_eq!(got_marker, marker);
            assert_eq!(got_body, body);
        }
    }

    #[test]
    fn extdata_marker_folders() {
        assert_eq!(Marker::Txt.folder(), Folder::Txt);
        assert_eq!(Marker::Dat.folder(), Folder::Dat);
        assert_eq!(Marker::Grp.folder(), Folder::Dat);
    }

    #[test]
    fn extdata_rejects_corruption() {
        // Too short.
        assert_eq!(unwrap(&[0u8; 10]), Err(StorageError::IllegalFormat));
        // Unknown marker.
        let mut bad = wrap(Marker::Txt, b"x");
        bad[0] = 0xFF;
        assert_eq!(unwrap(&bad), Err(StorageError::IllegalFormat));
        // Flipped body byte fails the HMAC.
        let mut tampered = wrap(Marker::Txt, b"hello");
        tampered[HEADER_LEN] ^= 0x01;
        assert_eq!(unwrap(&tampered), Err(StorageError::IllegalFormat));
    }

    #[test]
    fn extdata_key_is_64_bytes() {
        // The exact lpp-3ds-sbfm key; one byte too long/short would break SB compatibility.
        assert_eq!(HMAC_KEY.len(), 64);
    }

    #[test]
    fn mem_storage_serialize_roundtrip() {
        let mut s = MemStorage::new();
        s.write("DEFAULT", Folder::Txt, "P", b"PRINT 1").unwrap();
        s.write("DEFAULT", Folder::Dat, "PIC", &[0u8, 1, 2, 255])
            .unwrap();
        s.write("GAME", Folder::Txt, "MAIN", b"X=1").unwrap();
        let blob = s.serialize();
        let back = MemStorage::deserialize(&blob).unwrap();
        assert_eq!(back.read("DEFAULT", Folder::Txt, "P").unwrap(), b"PRINT 1");
        assert_eq!(
            back.read("DEFAULT", Folder::Dat, "PIC").unwrap(),
            [0, 1, 2, 255]
        );
        assert_eq!(back.read("GAME", Folder::Txt, "MAIN").unwrap(), b"X=1");
        assert_eq!(back.projects().unwrap(), s.projects().unwrap());
        // Equal stores serialize to equal bytes (deterministic).
        assert_eq!(back.serialize(), blob);
    }

    #[test]
    fn mem_storage_empty_serialize_roundtrip() {
        let blob = MemStorage::new().serialize();
        assert_eq!(
            MemStorage::deserialize(&blob).unwrap().projects().unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn mem_storage_deserialize_rejects_truncated() {
        let mut s = MemStorage::new();
        s.write("P", Folder::Txt, "A", b"hello").unwrap();
        let mut blob = s.serialize();
        blob.truncate(blob.len() - 2);
        assert_eq!(
            MemStorage::deserialize(&blob),
            Err(StorageError::IllegalFormat)
        );
        // Trailing garbage is also rejected.
        let mut extra = s.serialize();
        extra.push(0);
        assert_eq!(
            MemStorage::deserialize(&extra),
            Err(StorageError::IllegalFormat)
        );
    }

    fn hex(s: &str) -> [u8; 20] {
        let mut out = [0u8; 20];
        for (i, b) in out.iter_mut().enumerate() {
            *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }
}
