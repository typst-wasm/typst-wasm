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
        self.evict();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::project_file_id;
    use crate::state::ResourceKind;

    fn origin(path: &str) -> FileOrigin {
        FileOrigin {
            kind: ResourceKind::Project,
            requested_path: path.into(),
            resolved_path: None,
            media_type: None,
        }
    }

    #[test]
    fn reset_drops_fetched_state_but_retains_source() {
        let id = project_file_id("main.typ").unwrap();
        let mut store = FileStore::new();
        store.begin(id);
        store.replace_source(id, Source::new(id, "= Main".into()));
        let _ = store.commit(id, Err("not found".into()), origin("main.typ"));

        store.reset();

        let current = store.begin(id).unwrap();
        assert_eq!(current.0.unwrap().text(), "= Main");
        assert!(current.1.is_none());
        assert!(store.origin(id).is_none());
    }

    #[test]
    fn begin_reuses_success_and_failure_until_reset() {
        let id = project_file_id("data.txt").unwrap();
        let mut store = FileStore::new();
        store.begin(id);
        let _ = store.commit(id, Ok(Bytes::new(vec![1, 2])), origin("data.txt"));
        assert_eq!(
            store.begin(id).unwrap().1.unwrap().unwrap().as_slice(),
            &[1, 2]
        );

        store.reset();
        store.begin(id);
        let _ = store.commit(id, Err("denied".into()), origin("data.txt"));
        assert_eq!(store.begin(id).unwrap().1.unwrap().unwrap_err(), "denied");
    }

    #[test]
    fn replacing_source_updates_retained_byte_accounting() {
        let id = project_file_id("main.typ").unwrap();
        let mut store = FileStore::new();
        store.begin(id);
        store.replace_source(id, Source::new(id, "12345".into()));
        assert_eq!(store.retained_source_bytes, 5);
        store.replace_source(id, Source::new(id, "12".into()));
        assert_eq!(store.retained_source_bytes, 2);
    }

    #[test]
    fn source_budget_is_not_exceeded_after_eviction() {
        let first = project_file_id("first.typ").unwrap();
        let second = project_file_id("second.typ").unwrap();
        let mut store = FileStore::new();
        store.source_budget = 3;
        store.begin(first);
        store.begin(second);
        store.replace_source(first, Source::new(first, "12".into()));
        store.replace_source(second, Source::new(second, "34".into()));

        assert!(store.retained_source_bytes <= store.source_budget);
        let retained: usize = store
            .slots
            .values()
            .filter_map(|slot| slot.source.as_ref().map(source_size))
            .sum();
        assert!(retained <= store.source_budget);
    }
}
