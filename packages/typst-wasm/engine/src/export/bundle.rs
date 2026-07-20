use typst_bundle::BundleOptions;
use typst_render::RenderOptions;
use typst_svg::SvgOptions;

use crate::exports::typst::engine::api::CompileFailure;
use crate::typst::engine::types::{BundleFile, CompileOptions, CompilePayload};

pub fn export(
    bundle: &typst_bundle::Bundle,
    options: &CompileOptions,
) -> Result<CompilePayload, CompileFailure> {
    let ppi = super::validate_ppi(options.ppi.unwrap_or(144.0))?;

    let export_options = BundleOptions {
        html: typst_html::HtmlOptions { pretty: false },
        pdf: super::pdf::bundle_pdf_options(options)?,
        png: RenderOptions {
            pixel_per_pt: typst::utils::Scalar::new((ppi / 72.0) as f64),
            render_bleed: false,
        },
        svg: SvgOptions {
            render_bleed: false,
            pretty: false,
        },
    };

    let files = typst_bundle::export(bundle, &export_options)
        .map_err(|diags| super::diags_to_failure(diags, "bundle export failed"))?
        .into_iter()
        .map(|(path, data)| BundleFile {
            path: path.get_without_slash().to_owned(),
            data: data.to_vec(),
            media_type: media_type_for_path(path.get_without_slash()),
        })
        .collect();

    Ok(CompilePayload::Bundle(files))
}

fn media_type_for_path(path: &str) -> Option<String> {
    let media_type = match path.rsplit_once('.')?.1.to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => return None,
    };

    Some(media_type.into())
}
