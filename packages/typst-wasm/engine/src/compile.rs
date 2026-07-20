use std::cell::RefCell;

use typst::foundations::{Dict, Str, Value};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt};

use crate::export::{build_dependencies, convert_diagnostics, export_paged};
use crate::exports::typst::engine::api::{CompileFailure, CompileOptions, CompileSuccess};
use crate::state::CompilerState;
use crate::typst::engine::types::CompileFormat;
use crate::world::CompileWorld;

pub fn compile(
    state: &RefCell<CompilerState>,
    options: CompileOptions,
) -> Result<CompileSuccess, CompileFailure> {
    let snapshot = {
        let state = state.borrow();

        CompileSnapshot {
            font_book: state.font_book.clone(),
            fonts: state.fonts.clone(),
            file_store: state.file_store.clone(),
        }
    };

    let main = resolve_main(options.main.as_deref())?;

    snapshot
        .file_store
        .lock()
        .expect("file store mutex poisoned")
        .reset();

    let format = options.format.unwrap_or(CompileFormat::Pdf);

    let library = build_compile_library(&options)?;

    let world = CompileWorld::new(
        library,
        snapshot.font_book,
        snapshot.fonts,
        main,
        snapshot.file_store,
    );

    let result = match format {
        CompileFormat::Html => compile_html(&world, &options),
        CompileFormat::Bundle => compile_bundle(&world, &options),
        _ => compile_paged(&world, &options),
    };

    // Fetched files and dependency tracing are compile-local. Dependencies have
    // already been copied into the compile result above.
    let _ = world.into_runtime();
    result
}

struct CompileSnapshot {
    font_book: LazyHash<typst::text::FontBook>,
    fonts: Vec<typst::text::Font>,
    file_store: std::sync::Arc<std::sync::Mutex<crate::file_store::FileStore>>,
}

fn resolve_main(override_path: Option<&str>) -> Result<typst::syntax::FileId, CompileFailure> {
    if let Some(path) = override_path {
        return crate::paths::project_file_id(path)
            .map_err(crate::diagnostics::operation_compile_failure);
    }

    Err(crate::diagnostics::simple_compile_failure(
        "no main Typst file has been configured",
    ))
}

fn build_compile_library(options: &CompileOptions) -> Result<LazyHash<Library>, CompileFailure> {
    let format = options.format.unwrap_or(CompileFormat::Pdf);

    let mut features = Vec::new();
    match format {
        CompileFormat::Html => features.push(Feature::Html),
        CompileFormat::Bundle => {
            features.push(Feature::Bundle);
            features.push(Feature::Html);
        }
        _ => {}
    }

    let mut builder = Library::builder();

    if !features.is_empty() {
        builder = builder.with_features(features.into_iter().collect::<Features>());
    }

    if let Some(inputs) = &options.inputs {
        let dict: Dict = inputs
            .iter()
            .map(|entry| {
                (
                    Str::from(entry.key.as_str()),
                    Value::Str(Str::from(entry.value.as_str())),
                )
            })
            .collect();

        builder = builder.with_inputs(dict);
    }

    Ok(LazyHash::new(builder.build()))
}

fn compile_paged(
    world: &CompileWorld,
    options: &CompileOptions,
) -> Result<CompileSuccess, CompileFailure> {
    let Warned { output, warnings } = typst::compile::<typst_layout::PagedDocument>(world);

    match output {
        Ok(document) => {
            let payload = export_paged(&document, options)?;
            let metadata = crate::metadata::extract_document_metadata(&document);
            let diagnostics = convert_diagnostics(world, &[], &warnings);
            let dependencies = build_dependencies(world);

            Ok(CompileSuccess {
                output: payload,
                diagnostics,
                metadata,
                dependencies,
            })
        }
        Err(errors) => {
            let diagnostics = convert_diagnostics(world, &errors, &warnings);
            let dependencies = build_dependencies(world);

            Err(CompileFailure {
                diagnostics,
                dependencies,
                message: Some("compilation failed".into()),
            })
        }
    }
}

fn compile_html(
    world: &CompileWorld,
    options: &CompileOptions,
) -> Result<CompileSuccess, CompileFailure> {
    let Warned { output, warnings } = typst::compile::<typst_html::HtmlDocument>(world);

    match output {
        Ok(document) => {
            let html_str = crate::export::export_html(&document, options)?;
            let metadata = crate::metadata::extract_document_metadata(&document);
            let diagnostics = convert_diagnostics(world, &[], &warnings);
            let dependencies = build_dependencies(world);

            Ok(CompileSuccess {
                output: html_str,
                diagnostics,
                metadata,
                dependencies,
            })
        }
        Err(errors) => {
            let diagnostics = convert_diagnostics(world, &errors, &warnings);
            let dependencies = build_dependencies(world);

            Err(CompileFailure {
                diagnostics,
                dependencies,
                message: Some("HTML compilation failed".into()),
            })
        }
    }
}

fn compile_bundle(
    world: &CompileWorld,
    options: &CompileOptions,
) -> Result<CompileSuccess, CompileFailure> {
    let Warned { output, warnings } = typst::compile::<typst_bundle::Bundle>(world);

    match output {
        Ok(document) => {
            let output = crate::export::export_bundle(&document, options)?;
            let diagnostics = convert_diagnostics(world, &[], &warnings);
            let dependencies = build_dependencies(world);

            Ok(CompileSuccess {
                output,
                diagnostics,
                // A bundle can contain multiple documents, so it has no single
                // document-metadata value to expose here.
                metadata: None,
                dependencies,
            })
        }
        Err(errors) => {
            let diagnostics = convert_diagnostics(world, &errors, &warnings);
            let dependencies = build_dependencies(world);

            Err(CompileFailure {
                diagnostics,
                dependencies,
                message: Some("bundle compilation failed".into()),
            })
        }
    }
}

use typst::diag::Warned;
