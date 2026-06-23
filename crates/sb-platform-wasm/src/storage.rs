//! Browser IndexedDB-backed [`Storage`](sb_core::storage::Storage) (M6-T1).
//!
//! SmileBASIC's storage layer must survive a page reload, and a browser's only durable
//! key-value store available to wasm is **IndexedDB**. But IndexedDB is asynchronous while the
//! VM (and the `Storage` trait) is synchronous, so the impl is a **mirror + persistence**:
//!
//! * an in-memory [`MemStorage`](sb_core::storage::MemStorage) mirror serves every synchronous
//!   trait call (read/write/list/…), exactly as on native;
//! * the whole store is serialized to one opaque blob ([`MemStorage::serialize`]) and shuttled
//!   to/from a single IndexedDB record. [`IdbStorage::hydrate`] loads it once at startup;
//!   every mutation re-persists asynchronously in the background.
//!
//! Keeping the (de)serialization in the wasm-safe `sb-core` core means the only browser-specific
//! code here is the IndexedDB request plumbing — the storage *logic* stays gate-tested. The
//! whole module is `wasm32`-only; the native host uses the filesystem (`sb-platform-native`).

#![cfg(target_arch = "wasm32")]

use sb_core::storage::{Folder, MemStorage, Storage, StorageError};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransactionMode};

/// The IndexedDB database + object-store + record names. One record (`STORE_KEY`) holds the
/// entire serialized store.
const DB_NAME: &str = "smilebasic";
const STORE_NAME: &str = "files";
const STORE_KEY: &str = "store";

/// An IndexedDB-backed [`Storage`]. The synchronous trait is served from an in-memory mirror;
/// mutations persist to IndexedDB in the background. Construct with [`IdbStorage::open`], which
/// opens the database and hydrates the mirror, then use it like any other `Storage`.
pub struct IdbStorage {
    mirror: MemStorage,
    db: Option<IdbDatabase>,
}

impl IdbStorage {
    /// A store with an empty mirror and no database yet (every trait call works in-memory;
    /// nothing persists until [`open`](Self::open) connects a database).
    pub fn new() -> Self {
        IdbStorage {
            mirror: MemStorage::new(),
            db: None,
        }
    }

    /// Open (creating if needed) the IndexedDB database and hydrate the mirror from it, then
    /// invoke `on_ready` with the connected store. Asynchronous: the database open and the
    /// initial load are IndexedDB requests, so the result is delivered via the callback rather
    /// than returned. Errors (no `window`, IndexedDB unavailable) are returned synchronously.
    pub fn open<F>(on_ready: F) -> Result<(), JsValue>
    where
        F: FnOnce(IdbStorage) + 'static,
    {
        let factory = web_sys::window()
            .and_then(|w| w.indexed_db().ok().flatten())
            .ok_or_else(|| JsValue::from_str("IndexedDB unavailable"))?;
        let request: IdbOpenDbRequest = factory.open_with_u32(DB_NAME, 1)?;

        // First-time / version bump: create the object store.
        let on_upgrade = Closure::once(Box::new(move |event: web_sys::Event| {
            if let Some(req) = event.target().and_then(|t| t.dyn_into::<IdbRequest>().ok()) {
                if let Ok(result) = req.result() {
                    if let Ok(db) = result.dyn_into::<IdbDatabase>() {
                        let _ = db.create_object_store(STORE_NAME);
                    }
                }
            }
        }) as Box<dyn FnOnce(web_sys::Event)>);
        request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
        on_upgrade.forget();

        // Open succeeded: pull the database handle, then hydrate the mirror from its one record.
        let on_open = Closure::once(Box::new(move |event: web_sys::Event| {
            let db = event
                .target()
                .and_then(|t| t.dyn_into::<IdbRequest>().ok())
                .and_then(|req| req.result().ok())
                .and_then(|res| res.dyn_into::<IdbDatabase>().ok());
            let Some(db) = db else {
                on_ready(IdbStorage::new());
                return;
            };
            hydrate(db, on_ready);
        }) as Box<dyn FnOnce(web_sys::Event)>);
        request.set_onsuccess(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        Ok(())
    }

    /// Re-serialize the mirror and write it to IndexedDB (fire-and-forget). Called after every
    /// mutation; a persistence failure is silently dropped (the in-memory state is still
    /// correct for this session — only durability is lost).
    fn persist(&self) {
        let Some(db) = &self.db else { return };
        let Ok(tx) = db.transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readwrite)
        else {
            return;
        };
        let Ok(store) = tx.object_store(STORE_NAME) else {
            return;
        };
        let bytes = self.mirror.serialize();
        let value = js_sys::Uint8Array::from(bytes.as_slice());
        let _ = store.put_with_key(value.as_ref(), &JsValue::from_str(STORE_KEY));
    }
}

impl Default for IdbStorage {
    fn default() -> Self {
        IdbStorage::new()
    }
}

/// Read the single serialized record back into a fresh mirror, then hand the connected store
/// to `on_ready`. A missing record (first run) yields an empty store.
fn hydrate<F>(db: IdbDatabase, on_ready: F)
where
    F: FnOnce(IdbStorage) + 'static,
{
    let load = |db: &IdbDatabase| -> Option<IdbRequest> {
        let tx = db
            .transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readonly)
            .ok()?;
        let store = tx.object_store(STORE_NAME).ok()?;
        store.get(&JsValue::from_str(STORE_KEY)).ok()
    };

    let Some(request) = load(&db) else {
        on_ready(IdbStorage {
            mirror: MemStorage::new(),
            db: Some(db),
        });
        return;
    };

    let finish: Rc<RefCell<Option<F>>> = Rc::new(RefCell::new(Some(on_ready)));
    let db_cell = Rc::new(db);

    let finish_cb = finish.clone();
    let db_cb = db_cell.clone();
    let on_loaded = Closure::once(Box::new(move |event: web_sys::Event| {
        let mirror = event
            .target()
            .and_then(|t| t.dyn_into::<IdbRequest>().ok())
            .and_then(|req| req.result().ok())
            .filter(|res| !res.is_undefined() && !res.is_null())
            .and_then(|res| res.dyn_into::<js_sys::Uint8Array>().ok())
            .and_then(|arr| MemStorage::deserialize(&arr.to_vec()).ok())
            .unwrap_or_default();
        if let Some(cb) = finish_cb.borrow_mut().take() {
            cb(IdbStorage {
                mirror,
                db: Some((*db_cb).clone()),
            });
        }
    }) as Box<dyn FnOnce(web_sys::Event)>);
    request.set_onsuccess(Some(on_loaded.as_ref().unchecked_ref()));
    on_loaded.forget();
}

// The synchronous trait is served entirely from the mirror; the three mutators also kick off a
// background IndexedDB persist so the change survives a reload.
impl Storage for IdbStorage {
    fn projects(&self) -> Result<Vec<String>, StorageError> {
        self.mirror.projects()
    }

    fn list(&self, project: &str, folder: Folder) -> Result<Vec<String>, StorageError> {
        self.mirror.list(project, folder)
    }

    fn exists(&self, project: &str, folder: Folder, name: &str) -> bool {
        self.mirror.exists(project, folder, name)
    }

    fn read(&self, project: &str, folder: Folder, name: &str) -> Result<Vec<u8>, StorageError> {
        self.mirror.read(project, folder, name)
    }

    fn write(
        &mut self,
        project: &str,
        folder: Folder,
        name: &str,
        body: &[u8],
    ) -> Result<(), StorageError> {
        self.mirror.write(project, folder, name, body)?;
        self.persist();
        Ok(())
    }

    fn delete(&mut self, project: &str, folder: Folder, name: &str) -> Result<bool, StorageError> {
        let existed = self.mirror.delete(project, folder, name)?;
        if existed {
            self.persist();
        }
        Ok(existed)
    }

    fn rename(
        &mut self,
        project: &str,
        folder: Folder,
        from: &str,
        to: &str,
    ) -> Result<(), StorageError> {
        self.mirror.rename(project, folder, from, to)?;
        self.persist();
        Ok(())
    }
}
