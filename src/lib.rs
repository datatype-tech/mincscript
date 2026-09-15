//! MincScript: a small programming language with a Logos lexer and Chumsky parser.

pub mod ast;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use ast::{Expr, Program, Stmt};
pub use diagnostic::{eprint_diagnostics, Diagnostic};
pub use lexer::tokenize;
pub use token::Token;

/// Lex and parse `source` into a [`Program`].
pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let (tokens, lex_errors) = tokenize(source);
    if !lex_errors.is_empty() {
        return Err(lex_errors);
    }
    parser::parse_tokens(source.len()..source.len(), tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BinOp;

    #[test]
    fn parses_let_with_precedence() {
        let program = parse("let x = 1 + 2 * 3;").expect("should parse");
        assert_eq!(
            program.stmts,
            vec![Stmt::Let {
                name: "x".into(),
                value: Expr::Binary {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Int(1)),
                    rhs: Box::new(Expr::Binary {
                        op: BinOp::Mul,
                        lhs: Box::new(Expr::Int(2)),
                        rhs: Box::new(Expr::Int(3)),
                    }),
                },
            }]
        );
    }

    #[test]
    fn parses_expression_statement() {
        let program = parse("-(1 - 2);").expect("should parse");
        assert_eq!(
            program.stmts,
            vec![Stmt::Expr(Expr::UnaryNeg(Box::new(Expr::Binary {
                op: BinOp::Sub,
                lhs: Box::new(Expr::Int(1)),
                rhs: Box::new(Expr::Int(2)),
            })))]
        );
    }

    #[test]
    fn empty_source_is_empty_program() {
        let program = parse("").expect("should parse");
        assert!(program.stmts.is_empty());
    }

    #[test]
    fn rejects_unrecognized_token() {
        let errors = parse("let x = @;").expect_err("should fail lexing");
        assert!(errors.iter().any(|e| e.message.contains("unrecognized")));
    }

    #[test]
    fn rejects_incomplete_let() {
        let errors = parse("let x = ;").expect_err("should fail parsing");
        assert!(!errors.is_empty());
    }
}
