use std::sync::{Arc, Mutex};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, Source, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use crate::dependencies::DependencyTrace;
use crate::file_store::FileStore;

pub struct CompileRuntime {
    pub dependencies: DependencyTrace,
}

pub struct CompileWorld {
    library: LazyHash<Library>,
    font_book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    runtime: Mutex<CompileRuntime>,
    file_store: Arc<Mutex<FileStore>>,
}

impl CompileWorld {
    pub fn new(
        library: LazyHash<Library>,
        font_book: LazyHash<FontBook>,
        fonts: Vec<Font>,
        main: FileId,
        file_store: Arc<Mutex<FileStore>>,
    ) -> Self {
        Self {
            library,
            font_book,
            fonts,
            main,
            file_store,
            runtime: Mutex::new(CompileRuntime {
                dependencies: DependencyTrace::default(),
            }),
        }
    }

    pub fn into_runtime(self) -> CompileRuntime {
        self.runtime
            .into_inner()
            .expect("compile runtime mutex poisoned")
    }

    pub fn dependencies(&self) -> DependencyTrace {
        self.runtime
            .lock()
            .expect("compile runtime mutex poisoned")
            .dependencies
            .clone()
    }
}

impl World for CompileWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.font_book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        loaders::load_source(self, id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        loaders::load_file(self, id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        date::host_today(offset)
    }
}

mod loaders {
    use super::*;

    use crate::state::{FileOrigin, ResourceKind};
    use crate::typst::engine::host;
    use crate::typst::engine::types::{FetchError, FetchRequest, FileKind};

    pub fn load_source(world: &CompileWorld, id: FileId) -> FileResult<Source> {
        let bytes = load_bytes(world, id)?;
        let text = std::str::from_utf8(bytes.as_slice())
            .map_err(|_| FileError::Other(Some("source file is not valid UTF-8".into())))?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);

        let mut store = world.file_store.lock().expect("file store mutex poisoned");
        let source = store.source(id);
        let mut source = source.unwrap_or_else(|| Source::new(id, text.to_owned()));
        if source.text() != text {
            source.replace(text);
        }
        store.replace_source(id, source.clone());
        Ok(source)
    }

    pub fn load_file(world: &CompileWorld, id: FileId) -> FileResult<Bytes> {
        load_bytes(world, id)
    }

    fn load_bytes(world: &CompileWorld, id: FileId) -> FileResult<Bytes> {
        let current = world
            .file_store
            .lock()
            .expect("file store mutex poisoned")
            .begin(id)
            .expect("file slot reserved");
        if let Some(bytes) = current.1 {
            record_origin(world, id);
            return bytes.map_err(|message| FileError::Other(Some(message.into())));
        }

        let requested_path = crate::paths::file_id_path(id);
        let kind = classify_file_kind(id);
        let origin = FileOrigin {
            kind: map_resource_kind(kind),
            requested_path: requested_path.clone(),
            resolved_path: None,
            media_type: None,
        };
        let fetched = host::fetch(&FetchRequest {
            path: requested_path,
            kind,
        });
        let (result, origin) = match fetched {
            Ok(fetched) => (
                Ok(Bytes::new(fetched.data)),
                FileOrigin {
                    resolved_path: fetched.resolved_path,
                    media_type: fetched.media_type,
                    ..origin
                },
            ),
            Err(error) => (Err(fetch_error_message(error)), origin),
        };
        let result = world
            .file_store
            .lock()
            .expect("file store mutex poisoned")
            .commit(id, result, origin);
        record_origin(world, id);
        result.map_err(|message| FileError::Other(Some(message.into())))
    }

    fn record_origin(world: &CompileWorld, id: FileId) {
        let origin = world
            .file_store
            .lock()
            .expect("file store mutex poisoned")
            .origin(id);
        if let Some(origin) = origin {
            world
                .runtime
                .lock()
                .expect("compile runtime mutex poisoned")
                .dependencies
                .record_origin(&origin);
        }
    }

    fn fetch_error_message(error: FetchError) -> String {
        match error {
            FetchError::NotFound => "resource not found".into(),
            FetchError::Denied => "resource access denied".into(),
            FetchError::Timeout => "resource fetch timed out".into(),
            FetchError::Unavailable => "resource loader unavailable".into(),
            FetchError::Other(message) => message,
        }
    }

    fn classify_file_kind(id: FileId) -> FileKind {
        match id.root() {
            VirtualRoot::Package(_) => FileKind::Package,
            _ => FileKind::Project,
        }
    }

    fn map_resource_kind(kind: FileKind) -> ResourceKind {
        match kind {
            FileKind::Project => ResourceKind::Project,
            FileKind::Package => ResourceKind::Package,
            FileKind::Url => ResourceKind::Url,
        }
    }
}

mod date {
    use super::*;

    pub fn host_today(offset: Option<Duration>) -> Option<Datetime> {
        let seconds = offset
            .map(|duration| duration.seconds())
            .and_then(|seconds| {
                if seconds.is_finite() && seconds.fract() == 0.0 {
                    Some(seconds as i64)
                } else {
                    None
                }
            });

        let date = crate::typst::engine::host::today(seconds)?;

        let year = i32::try_from(date.year).ok()?;

        Datetime::from_ymd(year, date.month, date.day)
    }
}
