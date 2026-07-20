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

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(kind: ResourceKind, requested_path: &str, resolved_path: Option<&str>) -> FileOrigin {
        FileOrigin {
            kind,
            requested_path: requested_path.into(),
            resolved_path: resolved_path.map(str::to_owned),
            media_type: None,
        }
    }

    #[test]
    fn deduplicates_origins_in_first_seen_order() {
        let first = origin(ResourceKind::Project, "main.typ", Some("main.typ"));
        let duplicate = first.clone();
        let package = origin(ResourceKind::Package, "@preview/pkg:1.0.0/lib.typ", None);
        let resolved_differently = origin(ResourceKind::Project, "main.typ", Some("/main.typ"));

        let mut trace = DependencyTrace::default();
        trace.record_origin(&first);
        trace.record_origin(&duplicate);
        trace.record_origin(&package);
        trace.record_origin(&resolved_differently);

        let entries = trace.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].requested_path, first.requested_path);
        assert_eq!(entries[1].requested_path, package.requested_path);
        assert_eq!(entries[2].resolved_path, resolved_differently.resolved_path);
    }
}
