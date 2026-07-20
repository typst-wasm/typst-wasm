use std::path::{Component, Path};

use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};

use crate::exports::typst::engine::api::OperationError;

pub fn project_file_id(input: &str) -> Result<FileId, OperationError> {
    let normalized = normalize_project_path(input)?;

    let virtual_path =
        VirtualPath::new(normalized).map_err(|err| OperationError::InvalidPath(err.to_string()))?;

    Ok(FileId::new(RootedPath::new(
        VirtualRoot::Project,
        virtual_path,
    )))
}

pub fn normalize_project_path(input: &str) -> Result<String, OperationError> {
    if input.trim().is_empty() {
        return Err(OperationError::InvalidPath("path cannot be empty".into()));
    }

    if input.contains('\\') {
        return Err(OperationError::InvalidPath("invalid project path".into()));
    }

    let path = Path::new(input);

    if path.is_absolute() {
        return Err(OperationError::InvalidPath(
            "absolute paths are not allowed".into(),
        ));
    }

    let mut parts = Vec::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| OperationError::InvalidPath("path is not valid UTF-8".into()))?;

                parts.push(part);
            }

            Component::CurDir => {}

            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(OperationError::InvalidPath(
                        "path escapes the project root".into(),
                    ));
                }
            }

            Component::RootDir | Component::Prefix(_) => {
                return Err(OperationError::InvalidPath("invalid project path".into()));
            }
        }
    }

    if parts.is_empty() {
        return Err(OperationError::InvalidPath("path cannot be empty".into()));
    }

    Ok(parts.join("/"))
}

pub fn file_id_path(id: FileId) -> String {
    let path = id.vpath().get_without_slash().to_string();

    match id.root() {
        VirtualRoot::Project => path,
        VirtualRoot::Package(package) => format!("{package}/{path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_project_paths() {
        assert_eq!(
            normalize_project_path("a/./tmp/../b.typ").unwrap(),
            "a/b.typ"
        );
        assert_eq!(normalize_project_path("a//b.typ").unwrap(), "a/b.typ");
    }

    #[test]
    fn rejects_paths_outside_the_project() {
        for input in ["", "  ", ".", "..", "../a", "a/../../b", "/a", "a\\b"] {
            assert!(normalize_project_path(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn project_file_id_round_trips_to_normalized_path() {
        let id = project_file_id("./src/../main.typ").unwrap();
        assert_eq!(file_id_path(id), "main.typ");
    }
}
