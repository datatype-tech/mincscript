use std::fmt;

/// A parsed MincScript program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

/// A single statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Expr(Expr),
}

/// Expression nodes. This is a starter AST; more forms can be added later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Int(i64),
    Ident(String),
    UnaryNeg(Box<Expr>),
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Program")?;
        for stmt in &self.stmts {
            write_stmt(f, stmt, 1)?;
        }
        Ok(())
    }
}

fn write_stmt(f: &mut fmt::Formatter<'_>, stmt: &Stmt, indent: usize) -> fmt::Result {
    let pad = "  ".repeat(indent);
    match stmt {
        Stmt::Let { name, value } => {
            writeln!(f, "{pad}Let {name} =")?;
            write_expr(f, value, indent + 1)
        }
        Stmt::Expr(expr) => {
            writeln!(f, "{pad}Expr")?;
            write_expr(f, expr, indent + 1)
        }
    }
}

fn write_expr(f: &mut fmt::Formatter<'_>, expr: &Expr, indent: usize) -> fmt::Result {
    let pad = "  ".repeat(indent);
    match expr {
        Expr::Int(n) => writeln!(f, "{pad}Int({n})"),
        Expr::Ident(name) => writeln!(f, "{pad}Ident({name})"),
        Expr::UnaryNeg(expr) => {
            writeln!(f, "{pad}UnaryNeg")?;
            write_expr(f, expr, indent + 1)
        }
        Expr::Binary { op, lhs, rhs } => {
            writeln!(f, "{pad}Binary({op})")?;
            write_expr(f, lhs, indent + 1)?;
            write_expr(f, rhs, indent + 1)
        }
    }
}
