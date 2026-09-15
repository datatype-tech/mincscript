use logos::Logos;

use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::Token;

/// Scan `source` into tokens. Unrecognized input becomes diagnostics.
pub fn tokenize(source: &str) -> (Vec<(Token, Span)>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    errors.extend(unterminated_block_comments(source));
    errors.extend(unterminated_strings(source));

    for (result, span) in Token::lexer(source).spanned() {
        match result {
            Ok(token) => tokens.push((token, span)),
            Err(()) => errors.push(Diagnostic::new(span, "unrecognized token")),
        }
    }

    (tokens, errors)
}

fn unterminated_block_comments(source: &str) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_string = true;
            i += 1;
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            let mut closed = false;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                errors.push(Diagnostic::new(
                    start..source.len(),
                    "unterminated block comment",
                ));
                break;
            }
            continue;
        }
        i += 1;
    }
    errors
}

fn unterminated_strings(source: &str) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = i.saturating_add(2);
            continue;
        }
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            let mut escape = false;
            let mut closed = false;
            while i < bytes.len() {
                let c = bytes[i];
                if escape {
                    escape = false;
                } else if c == b'\\' {
                    escape = true;
                } else if c == b'"' {
                    closed = true;
                    i += 1;
                    break;
                } else if c == b'\n' {
                    break;
                }
                i += 1;
            }
            if !closed {
                errors.push(Diagnostic::new(start..i, "unterminated string"));
            }
            continue;
        }
        i += 1;
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Token;

    fn kinds(source: &str) -> Vec<Token> {
        let (tokens, errors) = tokenize(source);
        assert!(errors.is_empty(), "{errors:?} for {source:?}");
        tokens.into_iter().map(|(t, _)| t).collect()
    }

    #[test]
    fn keywords_are_not_idents() {
        let tokens = kinds("pack public class int boolean foreach chain world");
        assert_eq!(
            tokens,
            vec![
                Token::Pack,
                Token::Public,
                Token::Class,
                Token::IntKw,
                Token::BooleanKw,
                Token::Foreach,
                Token::Chain,
                Token::World,
            ]
        );
    }

    #[test]
    fn pack_path_and_annotation() {
        let tokens = kinds(r#"pack metro.escape; @OnLoad @Chain("Play")"#);
        assert_eq!(
            tokens,
            vec![
                Token::Pack,
                Token::Ident("metro".into()),
                Token::Dot,
                Token::Ident("escape".into()),
                Token::Semicolon,
                Token::At,
                Token::Ident("OnLoad".into()),
                Token::At,
                Token::Ident("Chain".into()),
                Token::LParen,
                Token::String("Play".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn compound_ops_and_range() {
        let tokens = kinds("a += 1; x in 1..10; f => g; h -> i;");
        assert!(tokens.contains(&Token::PlusEq));
        assert!(tokens.contains(&Token::DotDot));
        assert!(tokens.contains(&Token::FatArrow));
        assert!(tokens.contains(&Token::Arrow));
        assert!(tokens.contains(&Token::In));
    }

    #[test]
    fn strings_comments_and_unicode() {
        let tokens = kinds("/* c */ \"撤离成功\\n\" 阶段");
        assert_eq!(
            tokens,
            vec![
                Token::String("撤离成功\n".into()),
                Token::Ident("阶段".into()),
            ]
        );
    }

    #[test]
    fn line_comment_skipped() {
        let tokens = kinds("int keys; // score\nboolean extracted;");
        assert_eq!(
            tokens,
            vec![
                Token::IntKw,
                Token::Ident("keys".into()),
                Token::Semicolon,
                Token::BooleanKw,
                Token::Ident("extracted".into()),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn rejects_stray_symbol() {
        let (_, errors) = tokenize("int x = $;");
        assert!(errors.iter().any(|e| e.message.contains("unrecognized")));
    }

    #[test]
    fn unterminated_block_comment() {
        let (_, errors) = tokenize("int x; /* never");
        assert!(errors
            .iter()
            .any(|e| e.message.contains("unterminated block comment")));
    }
}
