use std::fmt;

use logos::Logos;

/// Lexical token for MincScript (Java-shaped, see `docs/language/`).
#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
pub enum Token {
    #[token("pack", priority = 2)]
    Pack,
    #[token("import", priority = 2)]
    Import,
    #[token("public", priority = 2)]
    Public,
    #[token("private", priority = 2)]
    Private,
    #[token("class", priority = 2)]
    Class,
    #[token("enum", priority = 2)]
    Enum,
    #[token("static", priority = 2)]
    Static,
    #[token("void", priority = 2)]
    Void,
    #[token("int", priority = 2)]
    IntKw,
    #[token("boolean", priority = 2)]
    BooleanKw,
    #[token("if", priority = 2)]
    If,
    #[token("else", priority = 2)]
    Else,
    #[token("foreach", priority = 2)]
    Foreach,
    #[token("for", priority = 2)]
    For,
    #[token("while", priority = 2)]
    While,
    #[token("switch", priority = 2)]
    Switch,
    #[token("case", priority = 2)]
    Case,
    #[token("default", priority = 2)]
    Default,
    #[token("return", priority = 2)]
    Return,
    #[token("new", priority = 2)]
    New,
    #[token("this", priority = 2)]
    This,
    #[token("true", priority = 2)]
    True,
    #[token("false", priority = 2)]
    False,
    #[token("as", priority = 2)]
    As,
    #[token("at", priority = 2)]
    AtKw,
    #[token("facing", priority = 2)]
    Facing,
    #[token("anchored", priority = 2)]
    Anchored,
    #[token("align", priority = 2)]
    Align,
    #[token("chain", priority = 2)]
    Chain,
    #[token("world", priority = 2)]
    World,
    #[token("place", priority = 2)]
    Place,
    #[token("chest", priority = 2)]
    Chest,
    #[token("barrel", priority = 2)]
    Barrel,
    #[token("fill", priority = 2)]
    Fill,
    #[token("set", priority = 2)]
    Set,
    #[token("cmd", priority = 2)]
    Cmd,
    #[token("temp", priority = 2)]
    Temp,
    #[token("run", priority = 2)]
    Run,
    #[token("include", priority = 2)]
    Include,
    #[token("link", priority = 2)]
    Link,
    #[token("clock", priority = 2)]
    Clock,
    #[token("host", priority = 2)]
    Host,
    #[token("origin", priority = 2)]
    Origin,
    #[token("dimension", priority = 2)]
    Dimension,
    #[token("tickingArea", priority = 2)]
    TickingArea,
    #[token("layout", priority = 2)]
    Layout,
    #[token("label", priority = 2)]
    Label,
    #[token("in", priority = 2)]
    In,
    #[token("with", priority = 2)]
    With,
    #[token("to", priority = 2)]
    To,
    #[token("from", priority = 2)]
    From,
    #[token("slot", priority = 2)]
    Slot,
    #[token("data", priority = 2)]
    Data,
    #[token("absolute", priority = 2)]
    Absolute,
    #[token("bound", priority = 2)]
    Bound,
    #[token("preload", priority = 2)]
    Preload,
    #[token("circle", priority = 2)]
    Circle,
    #[token("replace", priority = 2)]
    Replace,
    #[token("final", priority = 2)]
    Final,
    #[token("extends", priority = 2)]
    Extends,
    #[token("null", priority = 2)]
    Null,
    #[token("try", priority = 2)]
    Try,
    #[token("catch", priority = 2)]
    Catch,
    #[token("interface", priority = 2)]
    Interface,
    #[token("abstract", priority = 2)]
    Abstract,
    #[token("synchronized", priority = 2)]
    Synchronized,

    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("+=")]
    PlusEq,
    #[token("++")]
    PlusPlus,
    #[token("--")]
    MinusMinus,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("=>")]
    FatArrow,
    #[token("->")]
    Arrow,
    #[token("..")]
    DotDot,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("=")]
    Eq,
    #[token("!")]
    Bang,
    #[token("%")]
    Percent,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("@")]
    At,
    #[token("?")]
    Question,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,

    #[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]
    Int(i64),

    #[regex(r#""([^"\\]|\\.)*""#, lex_string)]
    String(String),

    /// ASCII + Unicode identifier (language spec: Unicode letters, no `$`).
    #[regex(r"[\p{XID_Start}_][\p{XID_Continue}]*", |lex| lex.slice().to_owned(), priority = 0)]
    Ident(String),
}

fn lex_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let slice = lex.slice();
    let inner = slice.get(1..slice.len().saturating_sub(1))?;
    unescape_string(inner)
}

fn unescape_string(s: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Token::Pack => "pack",
            Token::Import => "import",
            Token::Public => "public",
            Token::Private => "private",
            Token::Class => "class",
            Token::Enum => "enum",
            Token::Static => "static",
            Token::Void => "void",
            Token::IntKw => "int",
            Token::BooleanKw => "boolean",
            Token::If => "if",
            Token::Else => "else",
            Token::Foreach => "foreach",
            Token::For => "for",
            Token::While => "while",
            Token::Switch => "switch",
            Token::Case => "case",
            Token::Default => "default",
            Token::Return => "return",
            Token::New => "new",
            Token::This => "this",
            Token::True => "true",
            Token::False => "false",
            Token::As => "as",
            Token::AtKw => "at",
            Token::Facing => "facing",
            Token::Anchored => "anchored",
            Token::Align => "align",
            Token::Chain => "chain",
            Token::World => "world",
            Token::Place => "place",
            Token::Chest => "chest",
            Token::Barrel => "barrel",
            Token::Fill => "fill",
            Token::Set => "set",
            Token::Cmd => "cmd",
            Token::Temp => "temp",
            Token::Run => "run",
            Token::Include => "include",
            Token::Link => "link",
            Token::Clock => "clock",
            Token::Host => "host",
            Token::Origin => "origin",
            Token::Dimension => "dimension",
            Token::TickingArea => "tickingArea",
            Token::Layout => "layout",
            Token::Label => "label",
            Token::In => "in",
            Token::With => "with",
            Token::To => "to",
            Token::From => "from",
            Token::Slot => "slot",
            Token::Data => "data",
            Token::Absolute => "absolute",
            Token::Bound => "bound",
            Token::Preload => "preload",
            Token::Circle => "circle",
            Token::Replace => "replace",
            Token::Final => "final",
            Token::Extends => "extends",
            Token::Null => "null",
            Token::Try => "try",
            Token::Catch => "catch",
            Token::Interface => "interface",
            Token::Abstract => "abstract",
            Token::Synchronized => "synchronized",
            Token::EqEq => "==",
            Token::NotEq => "!=",
            Token::Le => "<=",
            Token::Ge => ">=",
            Token::AndAnd => "&&",
            Token::OrOr => "||",
            Token::PlusEq => "+=",
            Token::PlusPlus => "++",
            Token::MinusMinus => "--",
            Token::MinusEq => "-=",
            Token::StarEq => "*=",
            Token::SlashEq => "/=",
            Token::PercentEq => "%=",
            Token::FatArrow => "=>",
            Token::Arrow => "->",
            Token::DotDot => "..",
            Token::Lt => "<",
            Token::Gt => ">",
            Token::Eq => "=",
            Token::Bang => "!",
            Token::Percent => "%",
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Star => "*",
            Token::Slash => "/",
            Token::Dot => ".",
            Token::Colon => ":",
            Token::At => "@",
            Token::Question => "?",
            Token::LParen => "(",
            Token::RParen => ")",
            Token::LBrace => "{",
            Token::RBrace => "}",
            Token::LBracket => "[",
            Token::RBracket => "]",
            Token::Comma => ",",
            Token::Semicolon => ";",
            Token::Int(n) => return write!(f, "{n}"),
            Token::String(s) => return write!(f, "\"{s}\""),
            Token::Ident(name) => return write!(f, "{name}"),
        };
        write!(f, "{s}")
    }
}
