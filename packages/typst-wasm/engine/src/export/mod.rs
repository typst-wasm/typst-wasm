mod bundle;
mod html;
mod pdf;
mod png;
mod svg;

use typst::diag::SourceDiagnostic;
use typst_layout::PagedDocument;

use crate::diagnostics::{convert_severity, convert_typst_diagnostic};
use crate::exports::typst::engine::api::CompileFailure;
use crate::typst::engine::types::{
    CompileFormat, CompileOptions, CompilePayload, Diagnostic, DiagnosticSeverity, LoadedFile,
};
use crate::world::CompileWorld;

pub fn export_paged(
    document: &PagedDocument,
    options: &CompileOptions,
) -> Result<CompilePayload, CompileFailure> {
    let format = options.format.unwrap_or(CompileFormat::Pdf);

    match format {
        CompileFormat::Pdf => pdf::export(document, options),
        CompileFormat::Png => png::export(document, options),
        CompileFormat::Svg => svg::export(document, options),
        CompileFormat::Html => Err(crate::diagnostics::simple_compile_failure(
            "HTML export requires compiling with the HTML target",
        )),
        CompileFormat::Bundle => Err(crate::diagnostics::simple_compile_failure(
            "bundle export requires compiling with the HTML target",
        )),
    }
}

pub fn selected_pages<'a>(
    document: &'a PagedDocument,
    options: &CompileOptions,
) -> Result<Vec<&'a typst_layout::Page>, CompileFailure> {
    let all_pages = document.pages();

    let Some(pages_str) = &options.pages else {
        return Ok(all_pages.iter().collect());
    };

    let ranges = parse_page_ranges(pages_str)?;

    Ok(all_pages
        .iter()
        .filter(|page| {
            ranges
                .iter()
                .any(|range| range.includes(page.number as usize))
        })
        .collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageRange {
    Exact(usize),
    From(usize),
    To(usize),
    Between(usize, usize),
}

impl PageRange {
    fn includes(self, page: usize) -> bool {
        match self {
            Self::Exact(number) => page == number,
            Self::From(start) => page >= start,
            Self::To(end) => page <= end,
            Self::Between(start, end) => page >= start && page <= end,
        }
    }
}

fn invalid_page_range(input: &str) -> CompileFailure {
    crate::diagnostics::simple_compile_failure(format!("invalid page range: {input}"))
}

fn parse_page_number(input: &str, range: &str) -> Result<usize, CompileFailure> {
    input
        .parse::<usize>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| invalid_page_range(range))
}

fn parse_single_range(input: &str) -> Result<PageRange, CompileFailure> {
    let input = input.trim();
    if input.is_empty() {
        return Err(invalid_page_range(input));
    }

    let parts: Vec<&str> = input.split('-').map(str::trim).collect();

    match parts.as_slice() {
        [single] => {
            let n = parse_page_number(single, input)?;
            Ok(PageRange::Exact(n))
        }
        [start, ""] => {
            let n = parse_page_number(start, input)?;
            Ok(PageRange::From(n))
        }
        ["", end] => {
            let n = parse_page_number(end, input)?;
            Ok(PageRange::To(n))
        }
        [start, end] => {
            let s = parse_page_number(start, input)?;
            let e = parse_page_number(end, input)?;
            if s > e {
                return Err(invalid_page_range(input));
            }
            Ok(PageRange::Between(s, e))
        }
        _ => Err(invalid_page_range(input)),
    }
}

pub fn parse_page_ranges(input: &str) -> Result<Vec<PageRange>, CompileFailure> {
    let ranges = input
        .split(',')
        .map(str::trim)
        .map(parse_single_range)
        .collect::<Result<Vec<_>, _>>()?;

    if ranges.is_empty() {
        return Err(invalid_page_range(input));
    }

    Ok(ranges)
}

pub fn diags_to_failure(
    diags: impl IntoIterator<Item = SourceDiagnostic>,
    message: &str,
) -> CompileFailure {
    let diagnostics: Vec<_> = diags
        .into_iter()
        .map(|d| {
            let sev = convert_severity(d.severity);
            Diagnostic {
                message: d.message.to_string(),
                severity: sev,
                file: None,
                line: None,
                column: None,
                start: None,
                end: None,
                formatted: format!("{}", d.message),
                hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
                trace: d.trace.iter().map(|t| t.v.to_string()).collect(),
            }
        })
        .collect();

    CompileFailure {
        diagnostics,
        dependencies: Vec::new(),
        message: Some(message.into()),
    }
}

pub fn export_html(
    document: &typst_html::HtmlDocument,
    options: &CompileOptions,
) -> Result<CompilePayload, CompileFailure> {
    html::export(document, options)
}

pub fn export_bundle(
    bundle: &typst_bundle::Bundle,
    options: &CompileOptions,
) -> Result<CompilePayload, CompileFailure> {
    bundle::export(bundle, options)
}

pub fn build_dependencies(world: &CompileWorld) -> Vec<LoadedFile> {
    world
        .dependencies()
        .entries()
        .iter()
        .map(|origin| LoadedFile {
            kind: match origin.kind {
                crate::state::ResourceKind::Project => {
                    crate::typst::engine::types::FileKind::Project
                }
                crate::state::ResourceKind::Package => {
                    crate::typst::engine::types::FileKind::Package
                }
                crate::state::ResourceKind::Url => crate::typst::engine::types::FileKind::Url,
            },
            path: origin.requested_path.clone(),
            resolved_path: origin.resolved_path.clone(),
            media_type: origin.media_type.clone(),
        })
        .collect()
}

pub fn convert_diagnostics(
    world: &CompileWorld,
    errors: &[SourceDiagnostic],
    warnings: &[SourceDiagnostic],
) -> Vec<crate::typst::engine::types::Diagnostic> {
    let mut result: Vec<_> = errors
        .iter()
        .map(|d| convert_typst_diagnostic(world, d, DiagnosticSeverity::Error))
        .collect();

    result.extend(
        warnings
            .iter()
            .map(|d| convert_typst_diagnostic(world, d, DiagnosticSeverity::Warning)),
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page_range_forms() {
        assert_eq!(
            parse_page_ranges("1, 3-, -4, 2-5").unwrap(),
            vec![
                PageRange::Exact(1),
                PageRange::From(3),
                PageRange::To(4),
                PageRange::Between(2, 5),
            ]
        );
    }

    #[test]
    fn rejects_invalid_page_ranges() {
        for input in [
            "", "0", "-0", "0-2", "5-2", "-", "1-2-3", "abc", "1,,2", "1,", ",1",
        ] {
            assert!(parse_page_ranges(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn page_ranges_have_inclusive_boundaries() {
        assert!(PageRange::Exact(2).includes(2));
        assert!(!PageRange::Exact(2).includes(1));
        assert!(PageRange::From(2).includes(usize::MAX));
        assert!(!PageRange::From(2).includes(1));
        assert!(PageRange::To(2).includes(1));
        assert!(!PageRange::To(2).includes(3));
        assert!(PageRange::Between(2, 4).includes(2));
        assert!(PageRange::Between(2, 4).includes(4));
        assert!(!PageRange::Between(2, 4).includes(5));
    }
}
