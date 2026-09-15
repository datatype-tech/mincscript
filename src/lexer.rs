use logos::Logos;

use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::Token;

/// Scan `source` into tokens. Unrecognized input becomes diagnostics.
pub fn tokenize(source: &str) -> (Vec<(Token, Span)>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    for (result, span) in Token::lexer(source).spanned() {
        match result {
            Ok(token) => tokens.push((token, span)),
            Err(()) => errors.push(Diagnostic::new(span, "unrecognized token")),
        }
    }

    (tokens, errors)
}
