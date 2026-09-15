use chumsky::prelude::*;
use chumsky::Stream;

use crate::ast::{BinOp, Expr, Program, Stmt};
use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::Token;

/// Parse a token stream into a [`Program`].
pub fn parse_tokens(eoi: Span, tokens: Vec<(Token, Span)>) -> Result<Program, Vec<Diagnostic>> {
    let stream = Stream::from_iter(eoi.clone(), tokens.into_iter());
    let (program, errors) = program_parser().parse_recovery(stream);
    if !errors.is_empty() {
        return Err(errors.into_iter().map(diagnostic_from_simple).collect());
    }
    program.ok_or_else(|| vec![Diagnostic::new(eoi, "failed to parse program")])
}

fn diagnostic_from_simple(error: Simple<Token>) -> Diagnostic {
    Diagnostic::new(error.span(), error.to_string())
}

fn expr_parser() -> impl Parser<Token, Expr, Error = Simple<Token>> {
    recursive(|expr| {
        let atom = select! {
            Token::Ident(name) => Expr::Ident(name),
            Token::Int(n) => Expr::Int(n),
        }
        .or(expr.delimited_by(just(Token::LParen), just(Token::RParen)));

        let unary = just(Token::Minus)
            .repeated()
            .then(atom)
            .map(|(negs, expr)| {
                negs.into_iter()
                    .fold(expr, |acc, _| Expr::UnaryNeg(Box::new(acc)))
            });

        let product = unary
            .clone()
            .then(
                choice((
                    just(Token::Star).to(BinOp::Mul),
                    just(Token::Slash).to(BinOp::Div),
                ))
                .then(unary)
                .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });

        let sum = product
            .clone()
            .then(
                choice((
                    just(Token::Plus).to(BinOp::Add),
                    just(Token::Minus).to(BinOp::Sub),
                ))
                .then(product)
                .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });

        sum.clone()
            .then(
                choice((
                    just(Token::EqEq).to(BinOp::Eq),
                    just(Token::NotEq).to(BinOp::Ne),
                    just(Token::Le).to(BinOp::Le),
                    just(Token::Ge).to(BinOp::Ge),
                    just(Token::Lt).to(BinOp::Lt),
                    just(Token::Gt).to(BinOp::Gt),
                ))
                .then(sum)
                .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
    })
}

fn program_parser() -> impl Parser<Token, Program, Error = Simple<Token>> {
    let ident = select! { Token::Ident(name) => name };

    let let_stmt = just(Token::Let)
        .ignore_then(ident)
        .then_ignore(just(Token::Eq))
        .then(expr_parser())
        .then_ignore(just(Token::Semicolon))
        .map(|(name, value)| Stmt::Let { name, value });

    let expr_stmt = expr_parser()
        .then_ignore(just(Token::Semicolon))
        .map(Stmt::Expr);

    choice((let_stmt, expr_stmt))
        .repeated()
        .then_ignore(end())
        .map(|stmts| Program { stmts })
}
