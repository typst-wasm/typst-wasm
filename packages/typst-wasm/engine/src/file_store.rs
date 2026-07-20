use std::collections::HashMap;

use typst::foundations::Bytes;
use typst::syntax::{FileId, Source};

use crate::state::FileOrigin;

const DEFAULT_SOURCE_BUDGET: usize = 32 * 1024 * 1024;

pub struct FileSlot {
    pub source: Option<Source>,
    pub bytes: Option<Result<Bytes, String>>,
    pub origin: Option<FileOrigin>,
    pub accessed: bool,
}

pub type CurrentFile = (Option<Source>, Option<Result<Bytes, String>>);

pub struct FileStore {
    pub slots: HashMap<FileId, FileSlot>,
    retained_source_bytes: usize,
    source_budget: usize,
}

impl FileStore {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
            retained_source_bytes: 0,
            source_budget: DEFAULT_SOURCE_BUDGET,
        }
    }

    pub fn reset(&mut self) {
        for slot in self.slots.values_mut() {
            slot.accessed = false;
            slot.bytes = None;
            slot.origin = None;
        }
        self.evict();
    }

    pub fn begin(&mut self, id: FileId) -> Option<CurrentFile> {
        let slot = self.slots.entry(id).or_insert_with(|| FileSlot {
            source: None,
            bytes: None,
            origin: None,
            accessed: false,
        });
        slot.accessed = true;
        slot.bytes
            .as_ref()
            .map(|bytes| (slot.source.clone(), Some(bytes.clone())))
            .or_else(|| Some((slot.source.clone(), None)))
    }

    pub fn commit(
        &mut self,
        id: FileId,
        bytes: Result<Bytes, String>,
        origin: FileOrigin,
    ) -> Result<Bytes, String> {
        let slot = self.slots.get_mut(&id).expect("file slot reserved");
        slot.bytes = Some(bytes.clone());
        slot.origin = Some(origin);
        bytes
    }

    pub fn source(&self, id: FileId) -> Option<Source> {
        self.slots.get(&id).and_then(|slot| slot.source.clone())
    }

    pub fn replace_source(&mut self, id: FileId, source: Source) {
        let slot = self.slots.get_mut(&id).expect("file slot reserved");
        self.retained_source_bytes = self
            .retained_source_bytes
            .saturating_sub(slot.source.as_ref().map_or(0, source_size));
        self.retained_source_bytes += source_size(&source);
        slot.source = Some(source);
    }

    pub fn origin(&self, id: FileId) -> Option<FileOrigin> {
        self.slots.get(&id).and_then(|slot| slot.origin.clone())
    }

    fn evict(&mut self) {
        if self.retained_source_bytes <= self.source_budget {
            return;
        }
        for slot in self.slots.values_mut() {
            if self.retained_source_bytes <= self.source_budget {
                break;
            }
            if let Some(source) = slot.source.take() {
                self.retained_source_bytes = self
                    .retained_source_bytes
                    .saturating_sub(source_size(&source));
            }
        }
    }
}

fn source_size(source: &Source) -> usize {
    source.text().len().max(1)
}
