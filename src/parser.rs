use crate::ast::{
    Annotation, AssignOp, BinOp, ChainDef, ClassDef, CompilationUnit, ContainerKind, ContextPrefix,
    EnumDef, Expr, ExprKind, FieldDef, Item, Member, MethodDef, Param, PathName, PlaceDef,
    PlaceStmt, SlotFill, Stmt, StmtKind, TypeRef, UnaryOp, Visibility, WorldClause, WorldDef,
};
use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::Token;

fn is_punct(tok: &Token) -> bool {
    matches!(
        tok,
        Token::EqEq
            | Token::NotEq
            | Token::Le
            | Token::Ge
            | Token::AndAnd
            | Token::OrOr
            | Token::PlusEq
            | Token::MinusEq
            | Token::StarEq
            | Token::SlashEq
            | Token::PercentEq
            | Token::FatArrow
            | Token::Arrow
            | Token::DotDot
            | Token::Lt
            | Token::Gt
            | Token::Eq
            | Token::Bang
            | Token::Percent
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Dot
            | Token::Colon
            | Token::At
            | Token::Tilde
            | Token::LParen
            | Token::RParen
            | Token::LBrace
            | Token::RBrace
            | Token::LBracket
            | Token::RBracket
            | Token::Comma
            | Token::Semicolon
            | Token::Int(_)
            | Token::String(_)
    )
}

struct Parser {
    tokens: Vec<(Token, Span)>,
    pos: usize,
    eoi: Span,
    errors: Vec<Diagnostic>,
}

/// Parse a token stream into a compilation unit.
pub fn parse_tokens(
    eoi: Span,
    tokens: Vec<(Token, Span)>,
) -> Result<CompilationUnit, Vec<Diagnostic>> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        eoi,
        errors: Vec::new(),
    };
    let unit = parser.parse_unit();
    if parser.pos < parser.tokens.len() && parser.errors.is_empty() {
        let span = parser.peek_span();
        parser.errors.push(Diagnostic::new(
            span,
            "unexpected token after compilation unit",
        ));
    }
    if parser.errors.is_empty() {
        Ok(unit)
    } else {
        Err(parser.errors)
    }
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| self.eoi.clone())
    }

    fn prev_end(&self) -> usize {
        if self.pos == 0 {
            return self.eoi.start;
        }
        self.tokens
            .get(self.pos - 1)
            .map(|(_, s)| s.end)
            .unwrap_or(self.eoi.end)
    }

    fn expr(&self, start: usize, kind: ExprKind) -> Expr {
        Expr::new(start..self.prev_end(), kind)
    }

    fn bin(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        let start = lhs.span.start;
        let end = rhs.span.end;
        Expr::new(
            start..end,
            ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        )
    }

    fn stmt(&self, start: usize, kind: StmtKind) -> Stmt {
        Stmt::new(start..self.prev_end(), kind)
    }

    fn bump(&mut self) -> Option<(Token, Span)> {
        if self.pos < self.tokens.len() {
            let item = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset).map(|(t, _)| t)
    }

    fn skip_class_modifiers(&mut self) {
        while matches!(self.peek(), Some(Token::Static | Token::Final)) {
            self.pos += 1;
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        let span = self.peek_span();
        self.errors.push(Diagnostic::new(span, message));
    }

    fn expect(&mut self, kind: Token, what: &str) -> bool {
        if self.peek() == Some(&kind) {
            self.pos += 1;
            true
        } else {
            self.error(format!("expected {what}"));
            false
        }
    }

    fn parse_unit(&mut self) -> CompilationUnit {
        let mut pack = PathName::new(vec!["_".into()]);
        let mut pack_span = 0..0;
        if self.peek() == Some(&Token::Pack) {
            let start = self.peek_span();
            self.pos += 1;
            pack = self.parse_path();
            pack_span = start.start..self.peek_span().start;
            self.expect(Token::Semicolon, "`;` after pack");
        } else {
            self.error("expected `pack` at start of file");
        }

        let mut imports = Vec::new();
        while self.peek() == Some(&Token::Import) {
            self.pos += 1;
            imports.push(self.parse_path());
            self.expect(Token::Semicolon, "`;` after import");
        }

        let mut items = Vec::new();
        while self.pos < self.tokens.len() && self.errors.is_empty() {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => break,
            }
        }
        CompilationUnit {
            pack,
            pack_span,
            file: None,
            imports,
            items,
        }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let annotations = self.parse_annotations();
        if self.peek() == Some(&Token::World)
            || (self.peek() == Some(&Token::Public) && self.looks_like_world())
        {
            return Some(Item::World(self.parse_world(annotations)));
        }
        if self.peek() == Some(&Token::Place) {
            return Some(Item::Place(self.parse_place()));
        }

        let vis = self.parse_vis();
        self.skip_class_modifiers();
        if matches!(
            self.peek(),
            Some(
                Token::Interface
                    | Token::Abstract
                    | Token::Synchronized
                    | Token::Try
                    | Token::Catch
            )
        ) {
            self.error(format!(
                "`{}` is not valid in MincScript v1",
                self.peek().unwrap()
            ));
            return None;
        }
        if self.peek() == Some(&Token::Enum) {
            return Some(Item::Enum(self.parse_enum(vis)));
        }
        if self.peek() == Some(&Token::Chain) {
            return Some(Item::Chain(self.parse_chain(annotations, vis)));
        }
        if self.peek() == Some(&Token::Class) {
            return Some(Item::Class(self.parse_class(annotations, vis)));
        }
        self.error("expected class, enum, chain, world, or place");
        None
    }

    fn looks_like_world(&self) -> bool {
        false
    }

    fn parse_annotations(&mut self) -> Vec<Annotation> {
        let mut out = Vec::new();
        while self.peek() == Some(&Token::At) {
            self.pos += 1;
            let name = match self.bump() {
                Some((Token::Ident(name), _)) => name,
                _ => {
                    self.error("expected annotation name");
                    break;
                }
            };
            let mut args = Vec::new();
            if self.peek() == Some(&Token::LParen) {
                self.pos += 1;
                if self.peek() != Some(&Token::RParen) {
                    loop {
                        args.push(self.parse_expr());
                        if self.peek() == Some(&Token::Comma) {
                            self.pos += 1;
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Token::RParen, "`)`");
            }
            out.push(Annotation { name, args });
        }
        out
    }

    fn parse_vis(&mut self) -> Visibility {
        match self.peek() {
            Some(Token::Public) => {
                self.pos += 1;
                Visibility::Public
            }
            Some(Token::Private) => {
                self.pos += 1;
                Visibility::Private
            }
            _ => Visibility::Public,
        }
    }

    fn parse_path(&mut self) -> PathName {
        let mut parts = Vec::new();
        let first = self.bump_name();
        if first.is_empty() {
            return PathName::new(vec!["_".into()]);
        }
        parts.push(first);
        while matches!(self.peek(), Some(Token::Dot | Token::Colon)) {
            self.pos += 1;
            let part = self.bump_name();
            if part.is_empty() {
                self.error("expected identifier after `.` or `:`");
                break;
            }
            parts.push(part);
        }
        PathName::new(parts)
    }

    fn parse_type(&mut self) -> TypeRef {
        match self.peek() {
            Some(Token::Void) => {
                self.pos += 1;
                TypeRef::Void
            }
            Some(Token::IntKw) => {
                self.pos += 1;
                if self.peek() == Some(&Token::LBracket) {
                    self.error("`int[]` is not valid (no arrays; use scores or chests)");
                    self.pos += 1;
                    if self.peek() == Some(&Token::RBracket) {
                        self.pos += 1;
                    }
                }
                TypeRef::Int
            }
            Some(Token::BooleanKw) => {
                self.pos += 1;
                TypeRef::Boolean
            }
            Some(Token::Ident(name)) if name == "Seq" => {
                self.pos += 1;
                self.expect(Token::Lt, "`<`");
                let inner = self.parse_type();
                self.expect(Token::Gt, "`>`");
                TypeRef::Seq(Box::new(inner))
            }
            Some(Token::Ident(_)) => TypeRef::Named(self.parse_path()),
            _ => {
                self.error("expected type");
                TypeRef::Void
            }
        }
    }

    fn parse_class(&mut self, annotations: Vec<Annotation>, vis: Visibility) -> ClassDef {
        self.expect(Token::Class, "`class`");
        let name_span = self.peek_span();
        let name = self.expect_ident();
        let extends = if self.peek() == Some(&Token::Extends) {
            self.pos += 1;
            Some(self.expect_ident())
        } else {
            None
        };
        self.expect(Token::LBrace, "`{`");
        let mut members = Vec::new();
        while self.peek() != Some(&Token::RBrace)
            && self.pos < self.tokens.len()
            && self.errors.is_empty()
        {
            members.push(self.parse_member());
        }
        self.expect(Token::RBrace, "`}`");
        ClassDef {
            annotations,
            vis,
            name,
            name_span,
            extends,
            members,
        }
    }

    fn parse_member(&mut self) -> Member {
        let start = self.peek_span().start;
        let annotations = self.parse_annotations();
        let vis = self.parse_vis();
        let is_static = if self.peek() == Some(&Token::Static) {
            self.pos += 1;
            true
        } else {
            false
        };
        let ty = self.parse_type();
        let name = self.expect_ident();
        if self.peek() == Some(&Token::LParen) {
            self.pos += 1;
            let mut params = Vec::new();
            if self.peek() != Some(&Token::RParen) {
                loop {
                    let pty = self.parse_type();
                    let pname = self.expect_ident();
                    params.push(Param {
                        ty: pty,
                        name: pname,
                    });
                    if self.peek() == Some(&Token::Comma) {
                        self.pos += 1;
                        continue;
                    }
                    break;
                }
            }
            self.expect(Token::RParen, "`)`");
            self.expect(Token::LBrace, "`{`");
            let body = self.parse_block_contents();
            self.expect(Token::RBrace, "`}`");
            Member::Method(MethodDef {
                annotations,
                vis,
                is_static,
                return_ty: ty,
                name,
                params,
                body,
                span: start..self.prev_end(),
            })
        } else {
            let init = if self.peek() == Some(&Token::Eq) {
                self.pos += 1;
                Some(self.parse_expr())
            } else {
                None
            };
            self.expect(Token::Semicolon, "`;` after field");
            Member::Field(FieldDef {
                vis,
                is_static,
                ty,
                name,
                init,
                span: start..self.prev_end(),
            })
        }
    }

    fn parse_enum(&mut self, vis: Visibility) -> EnumDef {
        self.expect(Token::Enum, "`enum`");
        let name = self.expect_ident();
        self.expect(Token::LBrace, "`{`");
        let mut variants = Vec::new();
        while self.peek() != Some(&Token::RBrace)
            && self.pos < self.tokens.len()
            && self.errors.is_empty()
        {
            variants.push(self.expect_ident());
            if self.peek() == Some(&Token::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.expect(Token::RBrace, "`}`");
        EnumDef {
            vis,
            name,
            variants,
        }
    }

    fn parse_chain(&mut self, annotations: Vec<Annotation>, vis: Visibility) -> ChainDef {
        self.expect(Token::Chain, "`chain`");
        let name = self.expect_ident();
        self.expect(Token::LBrace, "`{`");
        let body = self.parse_block_contents();
        self.expect(Token::RBrace, "`}`");
        ChainDef {
            annotations,
            vis,
            name,
            body,
        }
    }

    fn parse_world(&mut self, annotations: Vec<Annotation>) -> WorldDef {
        if self.peek() == Some(&Token::Public) {
            self.pos += 1;
        }
        self.expect(Token::World, "`world`");
        let name = self.expect_ident();
        self.expect(Token::LBrace, "`{`");
        let mut clauses = Vec::new();
        while self.peek() != Some(&Token::RBrace)
            && self.pos < self.tokens.len()
            && self.errors.is_empty()
        {
            clauses.push(self.parse_world_clause());
        }
        self.expect(Token::RBrace, "`}`");
        WorldDef {
            annotations,
            name,
            clauses,
        }
    }

    fn parse_world_clause(&mut self) -> WorldClause {
        match self.peek() {
            Some(Token::Origin) => {
                self.pos += 1;
                let absolute = if self.peek() == Some(&Token::Absolute) {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                let coords = self.parse_coord_tuple();
                self.expect(Token::Semicolon, "`;`");
                WorldClause::Origin { absolute, coords }
            }
            Some(Token::Dimension) => {
                self.pos += 1;
                let name = self.expect_ident();
                self.expect(Token::Semicolon, "`;`");
                WorldClause::Dimension(name)
            }
            Some(Token::TickingArea) => {
                self.pos += 1;
                let name = match self.bump() {
                    Some((Token::String(s), _)) => s,
                    _ => {
                        self.error("expected ticking area name string");
                        String::new()
                    }
                };
                let mut radius = None;
                let mut preload = false;
                if self.peek() == Some(&Token::Circle) {
                    self.pos += 1;
                }
                let center = self.parse_coord_tuple();
                if matches!(self.peek(), Some(Token::Ident(s)) if s == "r") {
                    self.pos += 1;
                    self.expect(Token::Eq, "`=`");
                    radius = Some(self.parse_expr());
                }
                if self.peek() == Some(&Token::Preload) {
                    self.pos += 1;
                    preload = true;
                }
                self.expect(Token::Semicolon, "`;`");
                WorldClause::TickingArea {
                    name,
                    center,
                    radius,
                    preload,
                }
            }
            Some(Token::Chain) => {
                self.pos += 1;
                let name = self.expect_ident();
                let mut absolute = false;
                let mut at = Vec::new();
                let mut layout = None;
                let mut facing = None;
                let mut bound = None;
                let mut pack_mode = None;
                if self.peek() == Some(&Token::AtKw) {
                    self.pos += 1;
                    if self.peek() == Some(&Token::Absolute) {
                        self.pos += 1;
                        absolute = true;
                    }
                    at = self.parse_coord_tuple();
                }
                if self.peek() == Some(&Token::Layout) {
                    self.pos += 1;
                    layout = Some(self.expect_ident());
                }
                if self.peek() == Some(&Token::Facing) {
                    self.pos += 1;
                    facing = Some(self.expect_ident());
                }
                if self.peek() == Some(&Token::Bound) {
                    self.pos += 1;
                    bound = Some(self.parse_coord_tuple());
                }
                if matches!(self.peek(), Some(Token::Ident(s)) if s == "uses") {
                    self.pos += 1;
                }
                if self.peek() == Some(&Token::Pack) {
                    self.pos += 1;
                    if self.peek() == Some(&Token::Dot) {
                        self.pos += 1;
                    }
                    pack_mode = Some(self.bump_name());
                }
                self.expect(Token::Semicolon, "`;`");
                WorldClause::ChainPlace {
                    name,
                    absolute,
                    at,
                    layout,
                    facing,
                    bound,
                    pack_mode,
                }
            }
            Some(Token::Clock) => {
                self.pos += 1;
                let name = self.expect_ident();
                self.expect(Token::Semicolon, "`;`");
                WorldClause::Clock(name)
            }
            Some(Token::Host) => {
                self.pos += 1;
                let _ = self.expect_ident();
                self.expect(Token::Eq, "`=`");
                let mut value = self.bump_name();
                if matches!(self.peek(), Some(Token::Ident(_))) {
                    value.push(' ');
                    value.push_str(&self.expect_ident());
                }
                self.expect(Token::Semicolon, "`;`");
                WorldClause::HostTick(value)
            }
            Some(Token::Include) => {
                self.pos += 1;
                self.expect(Token::Place, "`place`");
                let name = self.expect_ident();
                self.expect(Token::Semicolon, "`;`");
                WorldClause::IncludePlace(name)
            }
            Some(Token::Link) => {
                self.pos += 1;
                let from_chain = self.expect_ident();
                let mut from_label = None;
                if self.peek() == Some(&Token::Dot) {
                    self.pos += 1;
                    from_label = Some(self.expect_ident());
                }
                self.expect(Token::FatArrow, "`=>`");
                let to = self.expect_ident();
                self.expect(Token::Semicolon, "`;`");
                WorldClause::Link {
                    from_chain,
                    from_label,
                    to,
                }
            }
            _ => {
                self.error("expected world clause");
                WorldClause::Clock(String::new())
            }
        }
    }

    fn parse_place(&mut self) -> PlaceDef {
        self.expect(Token::Place, "`place`");
        let name = self.expect_ident();
        self.expect(Token::LBrace, "`{`");
        let mut stmts = Vec::new();
        while self.peek() != Some(&Token::RBrace)
            && self.pos < self.tokens.len()
            && self.errors.is_empty()
        {
            stmts.push(self.parse_place_stmt());
        }
        self.expect(Token::RBrace, "`}`");
        PlaceDef { name, stmts }
    }

    fn parse_place_stmt(&mut self) -> PlaceStmt {
        match self.peek() {
            Some(Token::Fill) => {
                self.pos += 1;
                let from = self.parse_coord_tuple();
                self.expect(Token::To, "`to`");
                let to = self.parse_coord_tuple();
                self.expect(Token::With, "`with`");
                let block = self.parse_path();
                let mut replace = None;
                if self.peek() == Some(&Token::Replace) {
                    self.pos += 1;
                    replace = Some(self.parse_path());
                }
                self.expect(Token::Semicolon, "`;`");
                PlaceStmt::Fill {
                    from,
                    to,
                    block,
                    replace,
                }
            }
            Some(Token::Set) => {
                self.pos += 1;
                let block = self.parse_path();
                self.expect(Token::AtKw, "`at`");
                let at = self.parse_coord_tuple();
                self.expect(Token::Semicolon, "`;`");
                PlaceStmt::Set { block, at }
            }
            Some(Token::Chest) | Some(Token::Barrel) => {
                let kind = if self.peek() == Some(&Token::Chest) {
                    ContainerKind::Chest
                } else {
                    ContainerKind::Barrel
                };
                self.pos += 1;
                self.parse_container(kind)
            }
            Some(Token::Ident(name))
                if ContainerKind::from_name(name).is_some()
                    && matches!(self.peek_at(1), Some(Token::Ident(_)))
                    && self.peek_at(2) == Some(&Token::AtKw) =>
            {
                let kind = ContainerKind::from_name(name).unwrap();
                self.pos += 1;
                self.parse_container(kind)
            }
            Some(Token::Ident(_)) => {
                let block = self.parse_path();
                self.expect(Token::AtKw, "`at`");
                let at = self.parse_coord_tuple();
                self.expect(Token::Semicolon, "`;`");
                PlaceStmt::BlockAt { block, at }
            }
            _ => {
                self.error("expected place statement");
                PlaceStmt::Set {
                    block: PathName::new(vec!["air".into()]),
                    at: Vec::new(),
                }
            }
        }
    }

    fn parse_container(&mut self, kind: ContainerKind) -> PlaceStmt {
        let name = self.expect_ident();
        self.expect(Token::AtKw, "`at`");
        let at = self.parse_coord_tuple();
        let mut facing = None;
        if self.peek() == Some(&Token::Facing) {
            self.pos += 1;
            facing = Some(self.expect_ident());
        }
        self.expect(Token::LBrace, "`{`");
        let mut slots = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.errors.is_empty() {
            slots.push(self.parse_slot());
        }
        self.expect(Token::RBrace, "`}`");
        PlaceStmt::Container {
            kind,
            name,
            at,
            facing,
            slots,
        }
    }

    fn parse_slot(&mut self) -> SlotFill {
        self.expect(Token::Slot, "`slot`");
        let slot = match self.bump() {
            Some((Token::Int(n), _)) => n,
            _ => {
                self.error("expected slot index");
                0
            }
        };
        self.expect(Token::Colon, "`:`");
        let item = self.parse_path();
        let mut count = 1;
        if self.peek() == Some(&Token::Star) {
            self.pos += 1;
            count = match self.bump() {
                Some((Token::Int(n), _)) => n,
                _ => 1,
            };
        }
        let mut data = None;
        let mut title = None;
        let mut pages = None;
        if self.peek() == Some(&Token::Data) {
            self.pos += 1;
            data = match self.bump() {
                Some((Token::Int(n), _)) => Some(n),
                _ => None,
            };
        }
        while matches!(self.peek(), Some(Token::Ident(_))) {
            let key = self.expect_ident();
            match key.as_str() {
                "title" => {
                    title = match self.bump() {
                        Some((Token::String(s), _)) => Some(s),
                        _ => None,
                    };
                }
                "pages" => {
                    pages = match self.bump() {
                        Some((Token::String(s), _)) => Some(s),
                        _ => None,
                    };
                }
                _ => {
                    self.error(format!("unexpected slot field `{key}`"));
                    break;
                }
            }
        }
        self.expect(Token::Semicolon, "`;`");
        SlotFill {
            slot,
            item,
            count,
            data,
            title,
            pages,
        }
    }

    fn parse_coord_tuple(&mut self) -> Vec<Expr> {
        self.expect(Token::LParen, "`(`");
        let mut coords = Vec::new();
        if self.peek() != Some(&Token::RParen) {
            loop {
                coords.push(self.parse_expr());
                if self.peek() == Some(&Token::Comma) {
                    self.pos += 1;
                    continue;
                }
                break;
            }
        }
        self.expect(Token::RParen, "`)`");
        coords
    }

    fn parse_block_contents(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while self.peek() != Some(&Token::RBrace)
            && self.pos < self.tokens.len()
            && self.errors.is_empty()
        {
            stmts.push(self.parse_stmt());
        }
        stmts
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        self.expect(Token::LBrace, "`{`");
        let stmts = self.parse_block_contents();
        self.expect(Token::RBrace, "`}`");
        stmts
    }

    fn looks_like_name_token(tok: Option<&Token>) -> bool {
        match tok {
            Some(Token::Ident(_)) => true,
            Some(tok)
                if !is_punct(tok)
                    && !matches!(
                        tok,
                        Token::If
                            | Token::Else
                            | Token::Foreach
                            | Token::Switch
                            | Token::Case
                            | Token::Default
                            | Token::Return
                            | Token::Class
                            | Token::Enum
                            | Token::Public
                            | Token::Private
                            | Token::Pack
                            | Token::Import
                    ) =>
            {
                true
            }
            _ => false,
        }
    }

    fn looks_like_local(&self) -> bool {
        let Some(type_len) = self.type_prefix_len() else {
            return false;
        };
        Self::looks_like_name_token(self.peek_at(type_len))
            && matches!(
                self.peek_at(type_len + 1),
                Some(Token::Eq | Token::Semicolon)
            )
    }

    fn type_prefix_len(&self) -> Option<usize> {
        match self.peek() {
            Some(Token::IntKw | Token::BooleanKw) => Some(1),
            Some(Token::Ident(name)) if name == "Seq" && self.peek_at(1) == Some(&Token::Lt) => {
                let mut i = 2;
                if matches!(
                    self.peek_at(i),
                    Some(Token::Ident(_) | Token::IntKw | Token::BooleanKw | Token::Void)
                ) {
                    i += 1;
                } else {
                    return Some(1);
                }
                if self.peek_at(i) == Some(&Token::Gt) {
                    Some(i + 1)
                } else {
                    Some(1)
                }
            }
            Some(Token::Ident(_)) => {
                let mut i = 1;
                while self.peek_at(i) == Some(&Token::Dot) {
                    i += 1;
                    if matches!(self.peek_at(i), Some(Token::Ident(_))) {
                        i += 1;
                    } else {
                        break;
                    }
                }
                Some(i)
            }
            _ => None,
        }
    }

    fn parse_local(&mut self) -> Stmt {
        let start = self.peek_span().start;
        let ty = self.parse_type();
        let name = self.bump_name();
        let init = if self.peek() == Some(&Token::Eq) {
            self.pos += 1;
            Some(self.parse_expr())
        } else {
            None
        };
        self.expect(Token::Semicolon, "`;` after local");
        self.stmt(start, StmtKind::Local { ty, name, init })
    }

    fn parse_stmt(&mut self) -> Stmt {
        let start = self.peek_span().start;
        if self.peek() == Some(&Token::At) {
            let annotations = self.parse_annotations();
            let inner = self.parse_stmt();
            return self.stmt(
                start,
                StmtKind::Annotated {
                    annotations,
                    inner: Box::new(inner),
                },
            );
        }
        if self.looks_like_local() {
            return self.parse_local();
        }
        match self.peek() {
            Some(Token::If) => self.parse_if(),
            Some(Token::Foreach) => self.parse_foreach(),
            Some(Token::Switch) => self.parse_switch(),
            Some(Token::Try) => {
                self.error("`try`/`catch` is not valid in MincScript");
                self.pos += 1;
                self.stmt(start, StmtKind::Return(None))
            }
            Some(Token::Return) => {
                self.pos += 1;
                let expr = if self.peek() == Some(&Token::Semicolon) {
                    None
                } else {
                    Some(self.parse_expr())
                };
                self.expect(Token::Semicolon, "`;`");
                self.stmt(start, StmtKind::Return(expr))
            }
            Some(Token::Label) => {
                self.pos += 1;
                let name = self.expect_ident();
                self.expect(Token::Semicolon, "`;`");
                self.stmt(start, StmtKind::Label(name))
            }
            Some(Token::As | Token::AtKw | Token::Facing | Token::Anchored | Token::Align) => {
                self.parse_context()
            }
            _ => {
                let expr = self.parse_expr();
                if let Some(op) = self.assign_op() {
                    self.pos += 1;
                    let value = self.parse_expr();
                    self.expect(Token::Semicolon, "`;`");
                    self.stmt(
                        start,
                        StmtKind::Assign {
                            target: expr,
                            op,
                            value,
                        },
                    )
                } else {
                    self.expect(Token::Semicolon, "`;`");
                    self.stmt(start, StmtKind::Expr(expr))
                }
            }
        }
    }

    fn assign_op(&self) -> Option<AssignOp> {
        match self.peek() {
            Some(Token::Eq) => Some(AssignOp::Eq),
            Some(Token::PlusEq) => Some(AssignOp::PlusEq),
            Some(Token::MinusEq) => Some(AssignOp::MinusEq),
            Some(Token::StarEq) => Some(AssignOp::StarEq),
            Some(Token::SlashEq) => Some(AssignOp::SlashEq),
            Some(Token::PercentEq) => Some(AssignOp::PercentEq),
            _ => None,
        }
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.peek_span().start;
        self.expect(Token::If, "`if`");
        self.expect(Token::LParen, "`(`");
        let cond = self.parse_expr();
        self.expect(Token::RParen, "`)`");
        let then_body = self.parse_block();
        let else_body = if self.peek() == Some(&Token::Else) {
            self.pos += 1;
            if self.peek() == Some(&Token::If) {
                Some(vec![self.parse_if()])
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };
        self.stmt(
            start,
            StmtKind::If {
                cond,
                then_body,
                else_body,
            },
        )
    }

    fn parse_foreach(&mut self) -> Stmt {
        let start = self.peek_span().start;
        self.expect(Token::Foreach, "`foreach`");
        self.expect(Token::LParen, "`(`");
        let ty = self.parse_type();
        let name = self.expect_ident();
        self.expect(Token::Colon, "`:`");
        let iter = self.parse_expr();
        self.expect(Token::RParen, "`)`");
        let body = self.parse_block();
        self.stmt(
            start,
            StmtKind::Foreach {
                ty,
                name,
                iter,
                body,
            },
        )
    }

    fn parse_switch(&mut self) -> Stmt {
        let start = self.peek_span().start;
        self.expect(Token::Switch, "`switch`");
        self.expect(Token::LParen, "`(`");
        let expr = self.parse_expr();
        self.expect(Token::RParen, "`)`");
        self.expect(Token::LBrace, "`{`");
        let mut arms = Vec::new();
        let mut default = None;
        while self.peek() != Some(&Token::RBrace) && self.errors.is_empty() {
            if self.peek() == Some(&Token::Case) {
                self.pos += 1;
                let pat = self.parse_expr();
                if self.peek() == Some(&Token::Arrow) || self.peek() == Some(&Token::FatArrow) {
                    self.pos += 1;
                } else {
                    self.error("expected `->` or `=>`");
                }
                let body = if self.peek() == Some(&Token::LBrace) {
                    self.parse_block()
                } else {
                    vec![self.parse_stmt()]
                };
                arms.push((pat, body));
            } else if self.peek() == Some(&Token::Default) {
                self.pos += 1;
                if self.peek() == Some(&Token::Arrow) || self.peek() == Some(&Token::FatArrow) {
                    self.pos += 1;
                } else {
                    self.error("expected `->` or `=>`");
                }
                default = Some(if self.peek() == Some(&Token::LBrace) {
                    self.parse_block()
                } else {
                    vec![self.parse_stmt()]
                });
            } else {
                self.error("expected `case` or `default`");
                break;
            }
        }
        self.expect(Token::RBrace, "`}`");
        self.stmt(
            start,
            StmtKind::Switch {
                expr,
                arms,
                default,
            },
        )
    }

    fn parse_context(&mut self) -> Stmt {
        let start = self.peek_span().start;
        let mut prefixes = Vec::new();
        loop {
            match self.peek() {
                Some(Token::As) => {
                    self.pos += 1;
                    self.expect(Token::LParen, "`(`");
                    let e = self.parse_expr();
                    self.expect(Token::RParen, "`)`");
                    prefixes.push(ContextPrefix::As(e));
                }
                Some(Token::AtKw) => {
                    self.pos += 1;
                    self.expect(Token::LParen, "`(`");
                    let first = self.parse_expr();
                    let e = if self.peek() == Some(&Token::Comma) {
                        self.pos += 1;
                        let y = self.parse_expr();
                        self.expect(Token::Comma, "`,`");
                        let z = self.parse_expr();
                        Expr::new(
                            first.span.start..z.span.end,
                            ExprKind::Call {
                                callee: Box::new(Expr::new(
                                    first.span.clone(),
                                    ExprKind::Field {
                                        base: Box::new(Expr::new(
                                            first.span.clone(),
                                            ExprKind::Ident("BlockPos".into()),
                                        )),
                                        name: "of".into(),
                                    },
                                )),
                                args: vec![first, y, z],
                            },
                        )
                    } else {
                        first
                    };
                    self.expect(Token::RParen, "`)`");
                    prefixes.push(ContextPrefix::At(e));
                }
                Some(Token::Facing) => {
                    self.pos += 1;
                    self.expect(Token::LParen, "`(`");
                    let e = self.parse_expr();
                    self.expect(Token::RParen, "`)`");
                    prefixes.push(ContextPrefix::Facing(e));
                }
                Some(Token::Anchored) => {
                    self.pos += 1;
                    self.expect(Token::LParen, "`(`");
                    let which = self.expect_ident();
                    self.expect(Token::RParen, "`)`");
                    prefixes.push(ContextPrefix::Anchored(which));
                }
                Some(Token::Align) => {
                    self.pos += 1;
                    self.expect(Token::LParen, "`(`");
                    let axes = self.expect_ident();
                    self.expect(Token::RParen, "`)`");
                    prefixes.push(ContextPrefix::Align(axes));
                }
                _ => break,
            }
        }
        let body = self.parse_block();
        self.stmt(start, StmtKind::Context { prefixes, body })
    }

    fn bump_name(&mut self) -> String {
        match self.bump() {
            Some((Token::Ident(name), _)) => name,
            Some((tok, _)) if !is_punct(&tok) => tok.to_string(),
            _ => {
                self.error("expected identifier");
                String::new()
            }
        }
    }

    fn expect_ident(&mut self) -> String {
        match self.bump() {
            Some((Token::Ident(name), _)) => name,
            _ => {
                self.error("expected identifier");
                String::new()
            }
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_in()
    }

    fn parse_in(&mut self) -> Expr {
        let mut lhs = self.parse_or();
        if self.peek() == Some(&Token::In) {
            self.pos += 1;
            let rhs = self.parse_or();
            lhs = Self::bin(BinOp::In, lhs, rhs);
        }
        lhs
    }

    fn parse_or(&mut self) -> Expr {
        let mut lhs = self.parse_and();
        while self.peek() == Some(&Token::OrOr) {
            self.pos += 1;
            let rhs = self.parse_and();
            lhs = Self::bin(BinOp::Or, lhs, rhs);
        }
        lhs
    }

    fn parse_and(&mut self) -> Expr {
        let mut lhs = self.parse_cmp();
        while self.peek() == Some(&Token::AndAnd) {
            self.pos += 1;
            let rhs = self.parse_cmp();
            lhs = Self::bin(BinOp::And, lhs, rhs);
        }
        lhs
    }

    fn parse_cmp(&mut self) -> Expr {
        let mut lhs = self.parse_range();
        let op = match self.peek() {
            Some(Token::EqEq) => Some(BinOp::Eq),
            Some(Token::NotEq) => Some(BinOp::Ne),
            Some(Token::Lt) => Some(BinOp::Lt),
            Some(Token::Gt) => Some(BinOp::Gt),
            Some(Token::Le) => Some(BinOp::Le),
            Some(Token::Ge) => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let rhs = self.parse_range();
            lhs = Self::bin(op, lhs, rhs);
        }
        lhs
    }

    fn parse_range(&mut self) -> Expr {
        let mut lhs = self.parse_sum();
        if self.peek() == Some(&Token::DotDot) {
            self.pos += 1;
            let rhs = self.parse_sum();
            lhs = Self::bin(BinOp::Range, lhs, rhs);
        }
        lhs
    }

    fn parse_sum(&mut self) -> Expr {
        let mut lhs = self.parse_product();
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.parse_product();
            lhs = Self::bin(op, lhs, rhs);
        }
        lhs
    }

    fn parse_product(&mut self) -> Expr {
        let mut lhs = self.parse_unary();
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Rem,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.parse_unary();
            lhs = Self::bin(op, lhs, rhs);
        }
        lhs
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek() {
            Some(Token::Minus) => {
                let start = self.peek_span().start;
                self.pos += 1;
                let expr = self.parse_unary();
                self.expr(
                    start,
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                )
            }
            Some(Token::Bang) => {
                let start = self.peek_span().start;
                self.pos += 1;
                let expr = self.parse_unary();
                self.expr(
                    start,
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                )
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    self.pos += 1;
                    let name = self.bump_name();
                    let start = expr.span.start;
                    expr = self.expr(
                        start,
                        ExprKind::Field {
                            base: Box::new(expr),
                            name,
                        },
                    );
                }
                Some(Token::LParen) => {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        loop {
                            args.push(self.parse_expr());
                            if self.peek() == Some(&Token::Comma) {
                                self.pos += 1;
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(Token::RParen, "`)`");
                    let start = expr.span.start;
                    expr = self.expr(
                        start,
                        ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                    );
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let start = self.peek_span().start;
        match self.bump() {
            Some((Token::Int(n), span)) => Expr::new(span, ExprKind::Int(n)),
            Some((Token::String(s), span)) => Expr::new(span, ExprKind::String(s)),
            Some((Token::True, span)) => Expr::new(span, ExprKind::Bool(true)),
            Some((Token::False, span)) => Expr::new(span, ExprKind::Bool(false)),
            Some((Token::This, span)) => Expr::new(span, ExprKind::This),
            Some((Token::Null, span)) => Expr::new(span, ExprKind::Null),
            Some((Token::Ident(name), span)) => Expr::new(span, ExprKind::Ident(name)),
            Some((Token::New, span)) => {
                let ty = self.parse_type();
                self.expect(Token::LParen, "`(` after `new`");
                let mut args = Vec::new();
                if self.peek() != Some(&Token::RParen) {
                    loop {
                        args.push(self.parse_expr());
                        if self.peek() == Some(&Token::Comma) {
                            self.pos += 1;
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Token::RParen, "`)`");
                self.expr(span.start, ExprKind::New { ty, args })
            }
            Some((Token::Cmd, span)) => {
                self.expect(Token::LParen, "`(`");
                let s = match self.bump() {
                    Some((Token::String(s), _)) => s,
                    _ => {
                        self.error("expected command string");
                        String::new()
                    }
                };
                self.expect(Token::RParen, "`)`");
                self.expr(
                    span.start,
                    ExprKind::Call {
                        callee: Box::new(Expr::new(span.clone(), ExprKind::Ident("cmd".into()))),
                        args: vec![Expr::new(span, ExprKind::String(s))],
                    },
                )
            }
            Some((Token::LParen, _)) => {
                let expr = self.parse_expr();
                self.expect(Token::RParen, "`)`");
                expr
            }
            _ => {
                self.error("expected expression");
                self.expr(start, ExprKind::Int(0))
            }
        }
    }
}
