use std::sync::{Arc, Mutex};

use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

use crate::file_store::FileStore;

#[derive(Clone, Debug)]
pub struct FileOrigin {
    pub kind: ResourceKind,
    pub requested_path: String,
    pub resolved_path: Option<String>,
    pub media_type: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Project,
    Package,
    Url,
}

pub struct CompilerState {
    pub fonts: Vec<Font>,
    pub font_book: LazyHash<FontBook>,
    pub file_store: Arc<Mutex<FileStore>>,
}

impl CompilerState {
    pub fn new() -> Self {
        Self {
            fonts: Vec::new(),
            font_book: LazyHash::new(FontBook::new()),
            file_store: Arc::new(Mutex::new(FileStore::new())),
        }
    }
}
