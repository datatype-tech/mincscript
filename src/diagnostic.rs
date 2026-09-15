use ariadne::{Color, Label, Report, ReportKind, Source};
use thiserror::Error;

use crate::span::Span;

/// A source-located diagnostic produced by lexing or parsing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub file: Option<String>,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            file: None,
        }
    }
}

/// Print diagnostics to stderr using ariadne.
pub fn eprint_diagnostics(filename: &str, source: &str, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        let span = diagnostic.span.clone();
        let _ = Report::build(ReportKind::Error, (filename, span.clone()))
            .with_message(&diagnostic.message)
            .with_label(
                Label::new((filename, span))
                    .with_message(&diagnostic.message)
                    .with_color(Color::Red),
            )
            .finish()
            .eprint((filename, Source::from(source)));
    }
}
