use std::collections::HashSet;

use crate::state::{FileOrigin, ResourceKind};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DependencyKey {
    kind: ResourceKind,
    requested_path: String,
    resolved_path: Option<String>,
}

#[derive(Clone, Default)]
pub struct DependencyTrace {
    seen: HashSet<DependencyKey>,
    ordered: Vec<FileOrigin>,
}

impl DependencyTrace {
    pub fn record_origin(&mut self, origin: &FileOrigin) {
        let key = DependencyKey {
            kind: origin.kind,
            requested_path: origin.requested_path.clone(),
            resolved_path: origin.resolved_path.clone(),
        };

        if self.seen.insert(key) {
            self.ordered.push(origin.clone());
        }
    }

    pub fn entries(&self) -> &[FileOrigin] {
        &self.ordered
    }
}
