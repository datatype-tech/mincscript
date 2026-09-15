use std::fmt;

use crate::span::Span;

/// A parsed MincScript compilation unit (`docs/language/`).
#[derive(Debug, Clone, PartialEq)]
pub struct CompilationUnit {
    pub pack: PathName,
    pub pack_span: Span,
    pub file: Option<String>,
    pub imports: Vec<PathName>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathName {
    pub parts: Vec<String>,
}

impl PathName {
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }

    pub fn dotted(&self) -> String {
        self.parts.join(".")
    }

    /// Block / item id as written (`gold_block` or `minecraft:gold_block`).
    pub fn minecraft_id(&self) -> String {
        match self.parts.as_slice() {
            [] => String::new(),
            [one] => one.clone(),
            [ns, rest @ ..] => format!("{ns}:{}", rest.join("/")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Class(ClassDef),
    Enum(EnumDef),
    Chain(ChainDef),
    World(WorldDef),
    Place(PlaceDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub annotations: Vec<Annotation>,
    pub vis: Visibility,
    pub name: String,
    pub name_span: Span,
    pub extends: Option<String>,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    Field(FieldDef),
    Method(MethodDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub vis: Visibility,
    pub is_static: bool,
    pub ty: TypeRef,
    pub name: String,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub annotations: Vec<Annotation>,
    pub vis: Visibility,
    pub is_static: bool,
    pub return_ty: TypeRef,
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub ty: TypeRef,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Void,
    Int,
    Boolean,
    Named(PathName),
    Seq(Box<TypeRef>),
    /// Compile-time list (`List.of(...)`). Unrolled; not a runtime array.
    List(Box<TypeRef>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub vis: Visibility,
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChainDef {
    pub annotations: Vec<Annotation>,
    pub vis: Visibility,
    pub name: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldDef {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub clauses: Vec<WorldClause>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldClause {
    Origin {
        absolute: bool,
        coords: Vec<Expr>,
    },
    Dimension(String),
    TickingArea {
        name: String,
        center: Vec<Expr>,
        radius: Option<Expr>,
        preload: bool,
    },
    ChainPlace {
        name: String,
        absolute: bool,
        at: Vec<Expr>,
        layout: Option<String>,
        facing: Option<String>,
        bound: Option<Vec<Expr>>,
        /// `"one"` (one CB per command) or `"function"` (single CB + function body).
        pack_mode: Option<String>,
    },
    Clock(String),
    HostTick(String),
    IncludePlace(String),
    Link {
        from_chain: String,
        from_label: Option<String>,
        to: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceDef {
    pub name: String,
    pub stmts: Vec<PlaceStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceStmt {
    BlockAt {
        block: PathName,
        at: Vec<Expr>,
    },
    Fill {
        from: Vec<Expr>,
        to: Vec<Expr>,
        block: PathName,
        replace: Option<PathName>,
    },
    Set {
        block: PathName,
        at: Vec<Expr>,
    },
    Container {
        kind: ContainerKind,
        name: String,
        at: Vec<Expr>,
        facing: Option<String>,
        slots: Vec<SlotFill>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Chest,
    Barrel,
    ShulkerBox,
    Hopper,
    Dispenser,
    Furnace,
}

impl ContainerKind {
    pub fn block_id(self) -> &'static str {
        match self {
            ContainerKind::Chest => "chest",
            ContainerKind::Barrel => "barrel",
            ContainerKind::ShulkerBox => "shulker_box",
            ContainerKind::Hopper => "hopper",
            ContainerKind::Dispenser => "dispenser",
            ContainerKind::Furnace => "furnace",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "chest" => ContainerKind::Chest,
            "barrel" => ContainerKind::Barrel,
            "shulker_box" => ContainerKind::ShulkerBox,
            "hopper" => ContainerKind::Hopper,
            "dispenser" => ContainerKind::Dispenser,
            "furnace" => ContainerKind::Furnace,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotFill {
    pub slot: i64,
    pub item: PathName,
    pub count: i64,
    pub data: Option<i64>,
    pub title: Option<String>,
    pub pages: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}

impl Stmt {
    pub fn new(span: Span, kind: StmtKind) -> Self {
        Self { span, kind }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Annotated {
        annotations: Vec<Annotation>,
        inner: Box<Stmt>,
    },
    Local {
        ty: TypeRef,
        name: String,
        init: Option<Expr>,
        /// One-shot local: inlined / scratch `#tN`, then destroyed. Never a user objective.
        is_temp: bool,
    },
    /// One command, one statement (`run "…"`, `/say hi`, `run give @a diamond 1`).
    Run {
        command: String,
    },
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    Foreach {
        ty: TypeRef,
        name: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Switch {
        expr: Expr,
        arms: Vec<(Expr, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    },
    Context {
        prefixes: Vec<ContextPrefix>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Label(String),
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextPrefix {
    As(Expr),
    At(Expr),
    Facing(Expr),
    Anchored(String),
    Align(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

impl Expr {
    pub fn new(span: Span, kind: ExprKind) -> Self {
        Self { span, kind }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    String(String),
    Bool(bool),
    Ident(String),
    This,
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Field {
        base: Box<Expr>,
        name: String,
    },
    New {
        ty: TypeRef,
        args: Vec<Expr>,
    },
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Range,
    In,
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AssignOp::Eq => "=",
            AssignOp::PlusEq => "+=",
            AssignOp::MinusEq => "-=",
            AssignOp::StarEq => "*=",
            AssignOp::SlashEq => "/=",
            AssignOp::PercentEq => "%=",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Range => "..",
            BinOp::In => "in",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for CompilationUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "pack {}", self.pack.dotted())?;
        for item in &self.items {
            match item {
                Item::Class(c) => writeln!(f, "class {} fields={}", c.name, c.members.len())?,
                Item::Enum(e) => writeln!(f, "enum {} variants={}", e.name, e.variants.len())?,
                Item::Chain(c) => writeln!(f, "chain {} stmts={}", c.name, c.body.len())?,
                Item::World(w) => writeln!(f, "world {} clauses={}", w.name, w.clauses.len())?,
                Item::Place(p) => writeln!(f, "place {} stmts={}", p.name, p.stmts.len())?,
            }
        }
        Ok(())
    }
}
