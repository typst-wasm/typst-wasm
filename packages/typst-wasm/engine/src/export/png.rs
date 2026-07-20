use typst_layout::PagedDocument;
use typst_render::RenderOptions;

use crate::exports::typst::engine::api::CompileFailure;
use crate::typst::engine::types::{BytePage, CompileOptions, CompilePayload};

pub fn export(
    document: &PagedDocument,
    options: &CompileOptions,
) -> Result<CompilePayload, CompileFailure> {
    let ppi = super::validate_ppi(options.ppi.unwrap_or(144.0))?;
    let pixel_per_pt = ppi / 72.0;

    use typst::utils::Scalar;

    let opts = RenderOptions {
        pixel_per_pt: Scalar::new(pixel_per_pt as f64),
        render_bleed: false,
    };

    let pages = super::selected_pages(document, options)?;

    let png_pages: Vec<BytePage> = pages
        .iter()
        .map(|page| {
            let pixmap = typst_render::render(page, &opts);
            let data = pixmap.encode_png().map_err(|error| {
                crate::diagnostics::simple_compile_failure(format!("PNG export failed: {error}"))
            })?;
            Ok(BytePage {
                page: page.number as u32,
                data,
            })
        })
        .collect::<Result<_, CompileFailure>>()?;

    Ok(CompilePayload::Png(png_pages))
}
