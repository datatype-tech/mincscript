//! Lower MincScript to edition-specific Minecraft commands (no leading `/`).
//!
//! Bedrock and Java each get their own execute/selector printer. They do not
//! share an execute walker (`docs/minecraft-commands/crosswalk/parser-implementation.md`).

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use crate::ast::{
    AssignOp, BinOp, ChainDef, ClassDef, CompilationUnit, ContextPrefix, Expr, ExprKind, Item,
    Member, MethodDef, Stmt, StmtKind, TypeRef, UnaryOp,
};
use crate::cmds::{self, ItemStack};
use crate::config::{Edition, MincConfig};
use crate::diagnostic::Diagnostic;
use crate::extract::{expr_int, BinaryInfo, SymbolKind};
use crate::isa;
use crate::layout::{ChainCmdMeta, Facing};

#[derive(Debug, Clone)]
pub struct Lowered {
    pub functions: Vec<LoweredFn>,
    pub chains: Vec<LoweredChain>,
    pub setup: Vec<String>,
    pub tick_paths: Vec<String>,
    pub load_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LoweredFn {
    pub path: String,
    pub commands: Vec<String>,
    pub on_tick: bool,
    pub on_load: bool,
}

#[derive(Debug, Clone)]
pub struct LoweredChain {
    pub name: String,
    pub commands: Vec<(String, ChainCmdMeta)>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct FieldMeta {
    class: String,
    name: String,
    short: String,
    kind: SymbolKind,
    is_static: bool,
}

struct Lower<'a> {
    config: &'a MincConfig,
    info: &'a BinaryInfo,
    fields: HashMap<(String, String), FieldMeta>,
    methods: HashMap<(String, String), &'a MethodDef>,
    classes: HashMap<String, &'a ClassDef>,
    enums: HashMap<String, Vec<String>>,
    consts: HashMap<String, ConstVal>,
    current_class: String,
    this_sel: String,
    bindings: HashMap<String, String>,
    locals_expr: HashMap<String, Expr>,
    lists: HashMap<String, Vec<Expr>>,
    temp_names: HashSet<String>,
    temp_consumed: HashSet<String>,
    errors: Vec<Diagnostic>,
    temp: Cell<u32>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum ConstVal {
    Int(i64),
    Bool(bool),
    Str(String),
    Region {
        x: i64,
        y: i64,
        z: i64,
        dx: i64,
        dy: i64,
        dz: i64,
    },
    Item(ItemStack),
    Pos(String),
    List(Vec<ConstVal>),
    Tag(String),
}

#[derive(Clone, Debug)]
struct Selector {
    var: String,
    args: Vec<(String, String)>,
    java_items: Vec<(String, i64)>,
}

impl Selector {
    fn simple(var: &str) -> Self {
        Self {
            var: var.to_string(),
            args: Vec::new(),
            java_items: Vec::new(),
        }
    }

    fn emit(&self) -> String {
        if self.args.is_empty() {
            self.var.clone()
        } else {
            let inner = self
                .args
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",");
            format!("{}[{inner}]", self.var)
        }
    }
}

pub fn lower_project(
    unit: &CompilationUnit,
    config: &MincConfig,
    info: &BinaryInfo,
) -> Result<Lowered, Vec<Diagnostic>> {
    let mut lower = Lower::new(unit, config, info);
    let out = lower.run(unit);
    if lower.errors.is_empty() {
        Ok(out)
    } else {
        Err(lower.errors)
    }
}

impl<'a> Lower<'a> {
    fn new(unit: &'a CompilationUnit, config: &'a MincConfig, info: &'a BinaryInfo) -> Self {
        let mut fields = HashMap::new();
        let mut methods = HashMap::new();
        let mut classes = HashMap::new();
        let mut enums = HashMap::new();
        let mut consts = HashMap::new();
        for item in &unit.items {
            match item {
                Item::Class(class) => {
                    classes.insert(class.name.clone(), class);
                    for member in &class.members {
                        match member {
                            Member::Field(f) => {
                                if let Some(sym) = info.field_id(&info.pack, &class.name, &f.name) {
                                    fields.insert(
                                        (class.name.clone(), f.name.clone()),
                                        FieldMeta {
                                            class: class.name.clone(),
                                            name: f.name.clone(),
                                            short: sym.id.clone(),
                                            kind: sym.kind,
                                            is_static: f.is_static,
                                        },
                                    );
                                }
                                if let Some(init) = &f.init {
                                    if let Some(v) = eval_const(init) {
                                        consts.insert(format!("{}.{}", class.name, f.name), v);
                                    }
                                }
                            }
                            Member::Method(m) => {
                                methods.insert((class.name.clone(), m.name.clone()), m);
                            }
                        }
                    }
                }
                Item::Enum(en) => {
                    enums.insert(en.name.clone(), en.variants.clone());
                }
                _ => {}
            }
        }
        Self {
            config,
            info,
            fields,
            methods,
            classes,
            enums,
            consts,
            current_class: String::new(),
            this_sel: "@s".into(),
            bindings: HashMap::new(),
            locals_expr: HashMap::new(),
            lists: HashMap::new(),
            temp_names: HashSet::new(),
            temp_consumed: HashSet::new(),
            errors: Vec::new(),
            temp: Cell::new(0),
        }
    }

    fn run(&mut self, unit: &'a CompilationUnit) -> Lowered {
        let mut functions = Vec::new();
        let mut chains = Vec::new();
        let mut tick_paths = Vec::new();
        let mut load_paths = Vec::new();

        let mut setup = Vec::new();
        for sym in &self.info.symbols {
            if sym.kind == SymbolKind::Objective {
                setup.push(format!("scoreboard objectives add {} dummy", sym.id));
            }
        }
        for f in self.fields.values() {
            if f.is_static && f.kind == SymbolKind::Objective {
                setup.push(format!(
                    "scoreboard players add {} {} 0",
                    self.config.edition.world_holder(),
                    f.short
                ));
            }
        }

        for item in &unit.items {
            if let Item::Class(class) = item {
                for member in &class.members {
                    if let Member::Method(method) = member {
                        if method.name == "of" && method.body.is_empty() {
                            continue;
                        }
                        if method.annotations.iter().any(|a| a.name == "BedrockOnly")
                            && self.config.edition == Edition::Java
                            || method.annotations.iter().any(|a| a.name == "JavaOnly")
                                && self.config.edition == Edition::Bedrock
                        {
                            continue;
                        }
                        self.current_class = class.name.clone();
                        self.this_sel = "@s".into();
                        self.bindings.clear();
                        self.locals_expr.clear();
                        self.lists.clear();
                        self.temp_names.clear();
                        self.temp_consumed.clear();
                        self.temp.set(0);
                        for p in &method.params {
                            if matches!(named(&p.ty).as_str(), "Player" | "Entity" | "Runner")
                                || self.classes.contains_key(&named(&p.ty))
                            {
                                self.bindings.insert(p.name.clone(), "@s".into());
                            }
                        }
                        let mut cmds = self.lower_block(&method.body);
                        if !method.is_static {
                            cmds.splice(0..0, self.instance_score_inits());
                        }
                        if cmds.len() as u32 > self.config.function_command_limit {
                            self.errors.push(Diagnostic::new(
                                method.span.clone(),
                                format!(
                                    "function {}.{} exceeds limits.function_command_limit",
                                    class.name, method.name
                                ),
                            ));
                        }
                        let on_tick = method.annotations.iter().any(|a| a.name == "OnTick");
                        let on_load = method.annotations.iter().any(|a| a.name == "OnLoad");
                        if on_load {
                            cmds.splice(0..0, setup.iter().cloned());
                        }
                        let path = self.fn_path(&class.name, &method.name);
                        if on_tick {
                            tick_paths.push(path.clone());
                        }
                        if on_load {
                            load_paths.push(path.clone());
                        }
                        functions.push(LoweredFn {
                            path,
                            commands: cmds,
                            on_tick,
                            on_load,
                        });
                    }
                }
            }
        }

        if load_paths.is_empty() && !setup.is_empty() {
            let path = self.fn_path("_boot", "setup");
            load_paths.push(path.clone());
            functions.push(LoweredFn {
                path,
                commands: setup.clone(),
                on_tick: false,
                on_load: true,
            });
        }

        for item in &unit.items {
            if let Item::Chain(chain) = item {
                self.current_class.clear();
                self.this_sel = "@s".into();
                self.bindings.clear();
                self.locals_expr.clear();
                self.lists.clear();
                self.temp_names.clear();
                self.temp_consumed.clear();
                self.temp.set(0);
                let commands = self.lower_chain(chain);
                chains.push(LoweredChain {
                    name: chain.name.clone(),
                    commands,
                });
            }
        }

        for f in &mut functions {
            f.commands = crate::opt::optimize(std::mem::take(&mut f.commands));
        }
        for c in &mut chains {
            c.commands = crate::opt::optimize_pairs(std::mem::take(&mut c.commands));
        }
        setup = crate::opt::optimize(setup);

        Lowered {
            functions,
            chains,
            setup,
            tick_paths,
            load_paths,
        }
    }

    fn instance_score_inits(&self) -> Vec<String> {
        let mut out = Vec::new();
        for f in self.fields.values() {
            if f.class == self.current_class && !f.is_static && f.kind == SymbolKind::Objective {
                out.push(format!(
                    "scoreboard players add {} {} 0",
                    self.this_sel, f.short
                ));
            }
        }
        out
    }

    fn fn_path(&self, class: &str, method: &str) -> String {
        let class = class.to_lowercase();
        let method = method.to_lowercase();
        match self.config.edition {
            Edition::Bedrock => {
                format!("{}/{class}/{method}", self.config.bedrock_fn_prefix())
            }
            Edition::Java => format!("{}:{class}/{method}", self.config.java_namespace()),
        }
    }

    fn lower_chain(&mut self, chain: &ChainDef) -> Vec<(String, ChainCmdMeta)> {
        let mut out = Vec::new();
        let chain_repeat = chain.annotations.iter().any(|a| a.name == "Repeat");
        let chain_impulse = chain.annotations.iter().any(|a| a.name == "Impulse");
        let always = chain.annotations.iter().any(|a| a.name == "AlwaysActive")
            || !chain.annotations.iter().any(|a| a.name == "NeedsRedstone");
        let mut pending_label: Option<String> = None;
        for raw in &chain.body {
            let (inner, mut meta) = peel_stmt_meta(raw);
            meta.always_active = always;
            if chain_repeat && out.is_empty() {
                meta.repeat = true;
            }
            if chain_impulse && out.is_empty() {
                meta.impulse = true;
            }
            match &inner.kind {
                StmtKind::Label(name) => {
                    pending_label = Some(name.clone());
                    for cmd in self.link_cmds(&chain.name, name) {
                        let mut m = meta.clone();
                        m.label = Some(name.clone());
                        out.push((cmd, m));
                    }
                }
                _ => {
                    let cmds = match &inner.kind {
                        StmtKind::Return(_) | StmtKind::Local { .. } => {
                            self.lower_block(std::slice::from_ref(&inner))
                        }
                        _ => {
                            let cmds = self.lower_stmt(&inner);
                            self.consume_temps_in_stmt(&inner);
                            cmds
                        }
                    };
                    for (i, cmd) in cmds.into_iter().enumerate() {
                        let mut m = meta.clone();
                        if i > 0 {
                            m.conditional = false;
                            m.delay = 0;
                            m.label = None;
                            m.at = None;
                        } else if let Some(lab) = pending_label.take() {
                            m.label = Some(lab);
                        }
                        out.push((cmd, m));
                    }
                }
            }
        }
        out
    }

    fn link_cmds(&self, chain: &str, label: &str) -> Vec<String> {
        let mut out = Vec::new();
        for link in &self.info.links {
            if link.from_chain == chain && link.from_label.as_deref() == Some(label) {
                let path = match self.config.edition {
                    Edition::Bedrock => format!(
                        "{}/chain/{}",
                        self.config.bedrock_fn_prefix(),
                        link.to.to_lowercase()
                    ),
                    Edition::Java => format!(
                        "{}:chain/{}",
                        self.config.java_namespace(),
                        link.to.to_lowercase()
                    ),
                };
                out.push(format!("function {path}"));
            }
        }
        out
    }

    fn lower_block(&mut self, stmts: &[Stmt]) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::If {
                cond,
                then_body,
                else_body: None,
            } = &stmts[i].kind
            {
                if is_bare_return(then_body) {
                    let rest = self.lower_block(&stmts[i + 1..]);
                    let mut inverted = Vec::new();
                    for part in flatten_or(cond) {
                        inverted.extend(self.cond_clauses(part, true));
                    }
                    out.extend(wrap_clauses(&inverted, rest));
                    break;
                }
            }
            match &stmts[i].kind {
                StmtKind::Annotated { inner, annotations } => {
                    let _ = annotations;
                    out.extend(self.lower_block(std::slice::from_ref(inner.as_ref())));
                }
                StmtKind::Local {
                    name,
                    init,
                    ty,
                    is_temp,
                } => {
                    self.bind_local(name, ty, init.as_ref(), *is_temp);
                }
                StmtKind::Label(_) => {}
                StmtKind::Return(_) => {
                    if self.config.edition == Edition::Java {
                        out.push("return".into());
                    }
                    break;
                }
                _ => {
                    out.extend(self.lower_stmt(&stmts[i]));
                    self.consume_temps_in_stmt(&stmts[i]);
                }
            }
            i += 1;
        }
        out
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> Vec<String> {
        match &stmt.kind {
            StmtKind::Annotated { inner, .. } => self.lower_stmt(inner),
            StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                let then_cmds = self.lower_block(then_body);
                let else_cmds = else_body
                    .as_ref()
                    .map(|b| self.lower_block(b))
                    .unwrap_or_default();
                self.emit_if(cond, then_cmds, else_cmds)
            }
            StmtKind::Foreach {
                name, iter, body, ..
            } => {
                if let Some(elems) = self.list_elems(iter) {
                    let mut out = Vec::new();
                    for elem in elems {
                        let prev_bind = self.bindings.remove(name);
                        let prev_const = self.consts.remove(name);
                        let prev_local = self.locals_expr.remove(name);
                        self.locals_expr.insert(name.clone(), elem.clone());
                        if let Some(c) = self.const_of(&elem) {
                            self.consts.insert(name.clone(), c);
                        }
                        if let Some(sel) = self.selector_of(&elem) {
                            self.bindings.insert(name.clone(), sel.emit());
                        }
                        out.extend(self.lower_block(body));
                        self.locals_expr.remove(name);
                        self.consts.remove(name);
                        self.bindings.remove(name);
                        if let Some(p) = prev_bind {
                            self.bindings.insert(name.clone(), p);
                        }
                        if let Some(p) = prev_const {
                            self.consts.insert(name.clone(), p);
                        }
                        if let Some(p) = prev_local {
                            self.locals_expr.insert(name.clone(), p);
                        }
                    }
                    return out;
                }
                let sel = self
                    .selector_of(iter)
                    .unwrap_or_else(|| Selector::simple("@a"));
                let prev = self.bindings.insert(name.clone(), "@s".into());
                let inner = self.lower_block(body);
                if let Some(p) = prev {
                    self.bindings.insert(name.clone(), p);
                } else {
                    self.bindings.remove(name);
                }
                let merged = merge_as_at(&inner);
                let mut clauses = vec![format!("as {}", sel.emit())];
                clauses.extend(self.java_item_clauses(&sel));
                wrap_clauses(&clauses, merged)
            }
            StmtKind::Context { prefixes, body } => {
                let mut clauses = Vec::new();
                for p in prefixes {
                    match p {
                        ContextPrefix::As(e) => {
                            let sel = self
                                .selector_of(e)
                                .map(|s| s.emit())
                                .unwrap_or_else(|| "@s".into());
                            if let ExprKind::Ident(n) = &e.kind {
                                self.bindings.insert(n.clone(), "@s".into());
                            }
                            if sel != "@s" {
                                clauses.push(format!("as {sel}"));
                            }
                        }
                        ContextPrefix::At(e) => {
                            if triple_ints(e).is_some() {
                                continue;
                            }
                            let sel = self
                                .selector_of(e)
                                .map(|s| s.emit())
                                .unwrap_or_else(|| "@s".into());
                            clauses.push(format!("at {sel}"));
                        }
                        ContextPrefix::Facing(e) => {
                            if let Some(pos) = self.pos_of(e) {
                                clauses.push(format!("facing {pos}"));
                            }
                        }
                        ContextPrefix::Anchored(w) => clauses.push(format!("anchored {w}")),
                        ContextPrefix::Align(a) => clauses.push(format!("align {a}")),
                    }
                }
                let inner = self.lower_block(body);
                wrap_clauses(&clauses, inner)
            }
            StmtKind::Switch {
                expr,
                arms,
                default,
            } => {
                let mut out = Vec::new();
                for (pat, body) in arms {
                    let cmp = Expr::new(
                        expr.span.clone(),
                        ExprKind::Binary {
                            op: BinOp::Eq,
                            lhs: Box::new(expr.clone()),
                            rhs: Box::new(pat.clone()),
                        },
                    );
                    let cmds = self.lower_block(body);
                    let cl = self.cond_clauses(&cmp, false);
                    out.extend(wrap_clauses(&cl, cmds));
                }
                if let Some(body) = default {
                    out.extend(self.lower_block(body));
                }
                out
            }
            StmtKind::Assign { target, op, value } => self.lower_assign(target, *op, value),
            StmtKind::Expr(expr) => self.lower_expr_stmt(expr),
            StmtKind::Run { command } => {
                let s = cmds::strip_slash(command);
                if let Err(e) = isa::check_command(self.config.edition, &s) {
                    self.errors.push(Diagnostic::new(stmt.span.clone(), e));
                }
                vec![s]
            }
            StmtKind::While { cond, body } => self.lower_while(cond, body),
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => self.lower_for(init.as_deref(), cond.as_ref(), step.as_ref(), body),
            StmtKind::Return(_) | StmtKind::Label(_) | StmtKind::Local { .. } => Vec::new(),
        }
    }

    fn lower_assign(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> Vec<String> {
        if let ExprKind::Ident(n) = &value.kind {
            if let Some(e) = self.locals_expr.get(n).cloned() {
                return self.lower_assign(target, op, &e);
            }
        }
        if let ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } = &value.kind
        {
            let t = self.lower_assign(target, op, then_expr);
            let e = self.lower_assign(target, op, else_expr);
            return self.emit_if(cond, t, e);
        }
        if let Some((holder, field)) = self.lvalue(target) {
            match field.kind {
                SymbolKind::Tag => {
                    let add = match &value.kind {
                        ExprKind::Bool(true) => true,
                        ExprKind::Bool(false) => false,
                        _ => true,
                    };
                    let verb = if add { "add" } else { "remove" };
                    return vec![format!("tag {holder} {verb} {}", field.short)];
                }
                SymbolKind::Objective => {
                    if let Some(n) = self.int_of(value) {
                        let verb = match op {
                            AssignOp::Eq => "set",
                            AssignOp::PlusEq => "add",
                            AssignOp::MinusEq => "remove",
                            _ => {
                                return self.op_assign(&holder, &field.short, op, value);
                            }
                        };
                        let n = if op == AssignOp::MinusEq {
                            n.unsigned_abs() as i64
                        } else {
                            n
                        };
                        return vec![format!(
                            "scoreboard players {verb} {holder} {} {n}",
                            field.short
                        )];
                    }
                    if is_score_arith(value) {
                        let before = self.temp.get();
                        if let Some((mut cmds, h, o)) = self.materialize_score(value) {
                            let mop = assign_mop(op);
                            cmds.push(format!(
                                "scoreboard players operation {holder} {} {mop} {h} {o}",
                                field.short
                            ));
                            self.reset_scratch(before, &mut cmds);
                            return cmds;
                        }
                    }
                    return self.op_assign(&holder, &field.short, op, value);
                }
                _ => {}
            }
        }
        Vec::new()
    }

    fn op_assign(&mut self, holder: &str, obj: &str, op: AssignOp, value: &Expr) -> Vec<String> {
        let mut out = Vec::new();
        if let Some((vholder, vfield)) = self.lvalue(value) {
            if vfield.kind == SymbolKind::Objective {
                let mop = match op {
                    AssignOp::Eq => "=",
                    AssignOp::PlusEq => "+=",
                    AssignOp::MinusEq => "-=",
                    AssignOp::StarEq => "*=",
                    AssignOp::SlashEq => "/=",
                    AssignOp::PercentEq => "%=",
                };
                if op == AssignOp::Eq {
                    out.push(format!(
                        "scoreboard players operation {holder} {obj} = {vholder} {}",
                        vfield.short
                    ));
                } else {
                    out.push(format!(
                        "scoreboard players operation {holder} {obj} {mop} {vholder} {}",
                        vfield.short
                    ));
                }
                return out;
            }
        }
        if let Some(n) = self.int_of(value) {
            let t = self.temp_holder();
            out.push(format!("scoreboard players set {t} mt {n}"));
            let mop = match op {
                AssignOp::Eq => "=",
                AssignOp::PlusEq => "+=",
                AssignOp::MinusEq => "-=",
                AssignOp::StarEq => "*=",
                AssignOp::SlashEq => "/=",
                AssignOp::PercentEq => "%=",
            };
            out.push(format!(
                "scoreboard players operation {holder} {obj} {mop} {t} mt"
            ));
            out.push(format!("scoreboard players reset {t} mt"));
        }
        out
    }

    fn reset_scratch(&self, before: u32, cmds: &mut Vec<String>) {
        let after = self.temp.get();
        for n in before..after {
            cmds.push(format!("scoreboard players reset #t{n} mt"));
        }
    }

    fn temp_holder(&self) -> String {
        let n = self.temp.get();
        self.temp.set(n + 1);
        format!("#t{n}")
    }

    fn materialize_score(&self, expr: &Expr) -> Option<(Vec<String>, String, String)> {
        if let Some((holder, field)) = self.lvalue(expr) {
            if field.kind == SymbolKind::Objective {
                return Some((Vec::new(), holder, field.short));
            }
        }
        if let Some(n) = self.int_of(expr) {
            let t = self.temp_holder();
            return Some((
                vec![format!("scoreboard players set {t} mt {n}")],
                t,
                "mt".into(),
            ));
        }
        match &expr.kind {
            ExprKind::Binary { op, lhs, rhs } if is_arith(*op) => {
                let (mut cmds, h1, o1) = self.materialize_score(lhs)?;
                let (c2, h2, o2) = self.materialize_score(rhs)?;
                cmds.extend(c2);
                let t = self.temp_holder();
                let mop = bin_mop(*op)?;
                cmds.push(format!("scoreboard players operation {t} mt = {h1} {o1}"));
                cmds.push(format!(
                    "scoreboard players operation {t} mt {mop} {h2} {o2}"
                ));
                Some((cmds, t, "mt".into()))
            }
            _ => None,
        }
    }

    fn lower_expr_stmt(&mut self, expr: &Expr) -> Vec<String> {
        match &expr.kind {
            ExprKind::Update { expr, delta, .. } => {
                let op = if *delta >= 0 {
                    AssignOp::PlusEq
                } else {
                    AssignOp::MinusEq
                };
                let n = Expr::new(
                    expr.span.clone(),
                    ExprKind::Int(delta.unsigned_abs() as i64),
                );
                self.lower_assign(expr, op, &n)
            }
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let t = self.lower_expr_stmt(then_expr);
                let e = self.lower_expr_stmt(else_expr);
                self.emit_if(cond, t, e)
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::Ident(name) = &callee.kind {
                    return self.lower_builtin_call(name, args, expr.span.clone());
                }
                if let ExprKind::Field { base, name } = &callee.kind {
                    if name == "of" {
                        return Vec::new();
                    }
                    if name == "add" || name == "remove" {
                        return self.lower_tag_mut(base, name, args);
                    }
                    if let ExprKind::Ident(recv) = &base.kind {
                        if recv == "World" && name == "gamerule" {
                            let k = string_lit(&args[0]).unwrap_or_default();
                            let v = if args.len() > 1 {
                                render_lit(&args[1])
                            } else {
                                String::new()
                            };
                            return vec![format!("gamerule {k} {v}")];
                        }
                    }
                    if name == "gamemode" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        let mode = enum_or_name(args.first()).to_lowercase();
                        return vec![cmds::gamemode(&mode, &sel)];
                    }
                    if name == "give" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        let mut item = args
                            .first()
                            .map(|e| self.item_of(e))
                            .unwrap_or_else(|| ItemStack::new("air"));
                        if let Some(n) = args.get(1).and_then(|e| self.int_of(e)) {
                            item.count = n;
                        }
                        return self.emit_cmd(
                            expr.span.clone(),
                            cmds::give(self.config.edition, &sel, &item),
                        );
                    }
                    if name == "kill" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        return self
                            .emit_cmd(expr.span.clone(), cmds::kill(self.config.edition, &sel));
                    }
                    if name == "clear" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        let item = args.first().map(|e| self.item_of(e));
                        let n = args.get(1).and_then(|e| self.int_of(e));
                        return self.emit_cmd(
                            expr.span.clone(),
                            cmds::clear(self.config.edition, &sel, item.as_ref(), n),
                        );
                    }
                    if name == "replaceItem" || name == "replaceitem" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        let slot = args
                            .first()
                            .and_then(string_lit)
                            .unwrap_or_else(|| enum_or_name(args.first()));
                        let item = args
                            .get(1)
                            .map(|e| self.item_of(e))
                            .unwrap_or_else(|| ItemStack::new("air"));
                        return self.emit_cmd(
                            expr.span.clone(),
                            cmds::replace_item(self.config.edition, &sel, &slot, &item),
                        );
                    }
                    if name == "effect" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        return self.lower_effect(&sel, args, expr.span.clone());
                    }
                    if name == "teleport" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        let pos = args
                            .first()
                            .and_then(|e| self.pos_of(e))
                            .unwrap_or_else(|| "~ ~ ~".into());
                        return vec![cmds::teleport(&sel, &pos)];
                    }
                    if let Some(owner) = self.call_owner(base) {
                        if let Some(method) = self
                            .methods
                            .get(&(owner.clone(), name.clone()))
                            .map(|m| (*m).clone())
                        {
                            return self.call_method(&owner, &method, base, args);
                        }
                    }
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn call_method(
        &mut self,
        class: &str,
        method: &MethodDef,
        recv: &Expr,
        args: &[Expr],
    ) -> Vec<String> {
        if method.name == "of" {
            return Vec::new();
        }
        let saved_class = self.current_class.clone();
        let saved_this = self.this_sel.clone();
        let saved_bind = self.bindings.clone();
        if let Some(sel) = self.selector_of(recv) {
            self.this_sel = sel.emit();
        }
        self.current_class = class.to_string();
        let mut baked = Vec::new();
        for (p, a) in method.params.iter().zip(args) {
            if let Some(v) = eval_const(a).or_else(|| {
                if let ExprKind::Field { base, name } = &a.kind {
                    if let ExprKind::Ident(c) = &base.kind {
                        return self.consts.get(&format!("{c}.{name}")).cloned();
                    }
                }
                None
            }) {
                self.consts.insert(p.name.clone(), v);
                baked.push(p.name.clone());
            }
            if let Some(sel) = self.selector_of(a) {
                self.bindings.insert(p.name.clone(), sel.emit());
            }
        }
        let cmds = if method.params.is_empty() {
            vec![format!("function {}", self.fn_path(class, &method.name))]
        } else {
            self.lower_block(&method.body)
        };
        for k in baked {
            self.consts.remove(&k);
        }
        self.current_class = saved_class;
        self.this_sel = saved_this;
        self.bindings = saved_bind;
        cmds
    }

    fn call_owner(&self, expr: &Expr) -> Option<String> {
        match &expr.kind {
            ExprKind::Ident(n) if self.classes.contains_key(n) => Some(n.clone()),
            ExprKind::Call { callee, .. } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    if name == "of" {
                        if let ExprKind::Ident(c) = &base.kind {
                            return Some(c.clone());
                        }
                    }
                }
                self.call_owner(callee)
            }
            ExprKind::Field { base, .. } => self.call_owner(base),
            ExprKind::This => Some(self.current_class.clone()),
            _ => None,
        }
    }

    fn emit_cmd(&mut self, span: crate::span::Span, result: Result<String, String>) -> Vec<String> {
        match result {
            Ok(s) => vec![s],
            Err(e) => {
                self.errors.push(Diagnostic::new(span, e));
                Vec::new()
            }
        }
    }

    fn sel_arg(&self, args: &[Expr], default: &str) -> String {
        args.first()
            .and_then(|e| self.selector_of(e))
            .map(|s| s.emit())
            .unwrap_or_else(|| default.to_string())
    }

    fn text_arg(args: &[Expr], i: usize) -> String {
        args.get(i)
            .and_then(|e| match &e.kind {
                ExprKind::Call { callee, args } => {
                    if let ExprKind::Field { name, .. } = &callee.kind {
                        if name == "raw" {
                            return string_lit(&args[0]);
                        }
                    }
                    string_lit(e)
                }
                _ => string_lit(e),
            })
            .or_else(|| args.get(i).map(enum_or_name_expr))
            .unwrap_or_default()
    }

    fn lower_effect(&mut self, sel: &str, args: &[Expr], span: crate::span::Span) -> Vec<String> {
        let name = enum_or_name(args.first()).to_lowercase();
        if name == "clear" || name.is_empty() {
            return self.emit_cmd(span, cmds::effect_clear(self.config.edition, sel, None));
        }
        let seconds = args.get(1).and_then(|e| self.int_of(e)).unwrap_or(30);
        let amp = args.get(2).and_then(|e| self.int_of(e)).unwrap_or(0);
        let hide = matches!(args.get(3).map(|e| &e.kind), Some(ExprKind::Bool(true)));
        self.emit_cmd(
            span,
            cmds::effect_give(self.config.edition, sel, &name, seconds, amp, hide),
        )
    }

    fn lower_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: crate::span::Span,
    ) -> Vec<String> {
        match name {
            "cmd" | "run" => {
                let s = cmds::strip_slash(&string_lit(&args[0]).unwrap_or_default());
                if let Err(e) = isa::check_command(self.config.edition, &s) {
                    self.errors.push(Diagnostic::new(span, e));
                }
                vec![s]
            }
            "title" => {
                let sel = self.sel_arg(args, "@a");
                let loc = enum_or_name(args.get(1)).to_lowercase();
                let text = Self::text_arg(args, 2);
                self.emit_cmd(span, cmds::title(self.config.edition, &sel, &loc, &text))
            }
            "tellraw" => {
                let sel = self.sel_arg(args, "@a");
                let text = Self::text_arg(args, 1);
                self.emit_cmd(span, cmds::tellraw(self.config.edition, &sel, &text))
            }
            "give" => {
                let sel = self.sel_arg(args, "@s");
                let mut item = args
                    .get(1)
                    .map(|e| self.item_of(e))
                    .unwrap_or_else(|| ItemStack::new("air"));
                if let Some(n) = args.get(2).and_then(|e| self.int_of(e)) {
                    item.count = n;
                }
                self.emit_cmd(span, cmds::give(self.config.edition, &sel, &item))
            }
            "kill" => {
                let sel = self.sel_arg(args, "@s");
                self.emit_cmd(span, cmds::kill(self.config.edition, &sel))
            }
            "effect" => {
                let sel = self.sel_arg(args, "@s");
                let rest = if args.len() > 1 { &args[1..] } else { &[] };
                self.lower_effect(&sel, rest, span)
            }
            "clear" => {
                let sel = self.sel_arg(args, "@s");
                let item = args.get(1).map(|e| self.item_of(e));
                let n = args.get(2).and_then(|e| self.int_of(e));
                self.emit_cmd(
                    span,
                    cmds::clear(self.config.edition, &sel, item.as_ref(), n),
                )
            }
            "playsound" => {
                let sound = Self::text_arg(args, 0);
                let sel = args
                    .get(1)
                    .and_then(|e| self.selector_of(e))
                    .map(|s| s.emit())
                    .unwrap_or_else(|| "@a".into());
                let pos = args.get(2).and_then(|e| self.pos_of(e));
                let vol = args.get(3).and_then(|e| self.int_of(e)).map(|n| n as f64);
                let pitch = args.get(4).and_then(|e| self.int_of(e)).map(|n| n as f64);
                self.emit_cmd(
                    span,
                    cmds::playsound(
                        self.config.edition,
                        &sound,
                        &sel,
                        pos.as_deref(),
                        vol,
                        pitch,
                        None,
                    ),
                )
            }
            "particle" => {
                let n = Self::text_arg(args, 0);
                let pos = args
                    .get(1)
                    .and_then(|e| self.pos_of(e))
                    .unwrap_or_else(|| "~ ~ ~".into());
                self.emit_cmd(span, cmds::particle(self.config.edition, &n, &pos))
            }
            "summon" => {
                let entity = Self::text_arg(args, 0);
                let pos = args
                    .get(1)
                    .and_then(|e| self.pos_of(e))
                    .unwrap_or_else(|| "~ ~ ~".into());
                let extra = args.get(2).and_then(string_lit);
                self.emit_cmd(
                    span,
                    cmds::summon(self.config.edition, &entity, &pos, extra.as_deref()),
                )
            }
            "setblock" => {
                let pos = args
                    .first()
                    .and_then(|e| self.pos_of(e))
                    .unwrap_or_else(|| "~ ~ ~".into());
                let block = args
                    .get(1)
                    .map(|e| self.block_of(e))
                    .unwrap_or_else(|| "air".into());
                self.emit_cmd(span, cmds::setblock(self.config.edition, &pos, &block))
            }
            "say" => vec![cmds::say(&Self::text_arg(args, 0))],
            "weather" => {
                let kind = enum_or_name(args.first()).to_lowercase();
                let d = args.get(1).and_then(|e| self.int_of(e));
                vec![cmds::weather(&kind, d)]
            }
            "time" => {
                let spec = args
                    .first()
                    .and_then(|e| self.int_of(e).map(|n| n.to_string()))
                    .unwrap_or_else(|| enum_or_name(args.first()).to_lowercase());
                vec![cmds::time_set(&spec)]
            }
            "xp" => {
                let sel = self.sel_arg(args, "@s");
                let amount = args.get(1).and_then(|e| self.int_of(e)).unwrap_or(1);
                let levels = matches!(args.get(2).map(|e| &e.kind), Some(ExprKind::Bool(true)))
                    || enum_or_name(args.get(2)).eq_ignore_ascii_case("levels");
                self.emit_cmd(span, cmds::xp(self.config.edition, &sel, amount, levels))
            }
            "difficulty" => vec![cmds::difficulty(&enum_or_name(args.first()))],
            "enchant" => {
                let sel = self.sel_arg(args, "@s");
                let ench = enum_or_name(args.get(1)).to_lowercase();
                let level = args.get(2).and_then(|e| self.int_of(e)).unwrap_or(1);
                self.emit_cmd(span, cmds::enchant(self.config.edition, &sel, &ench, level))
            }
            "replaceItem" | "replaceitem" => {
                let sel = self.sel_arg(args, "@s");
                let slot = args
                    .get(1)
                    .and_then(string_lit)
                    .unwrap_or_else(|| enum_or_name(args.get(1)));
                let item = args
                    .get(2)
                    .map(|e| self.item_of(e))
                    .unwrap_or_else(|| ItemStack::new("air"));
                self.emit_cmd(
                    span,
                    cmds::replace_item(self.config.edition, &sel, &slot, &item),
                )
            }
            "random" => {
                if self.config.edition == Edition::Java {
                    self.errors
                        .push(Diagnostic::new(span, "`random` is Bedrock-only"));
                    return Vec::new();
                }
                let lo = args.first().and_then(|e| self.int_of(e)).unwrap_or(0);
                let hi = args.get(1).and_then(|e| self.int_of(e)).unwrap_or(0);
                vec![format!(
                    "scoreboard players random {} mt {lo} {hi}",
                    self.this_sel
                )]
            }
            _ => Vec::new(),
        }
    }

    fn bind_local(&mut self, name: &str, ty: &TypeRef, init: Option<&Expr>, is_temp: bool) {
        if is_temp {
            self.temp_names.insert(name.to_string());
        }
        let Some(init) = init else {
            return;
        };
        self.locals_expr.insert(name.to_string(), init.clone());
        if let Some(elems) = self.list_elems(init) {
            self.lists.insert(name.to_string(), elems);
        }
        if let Some(v) = self.const_of(init) {
            self.consts.insert(name.to_string(), v);
        }
        if matches!(named(ty).as_str(), "Player" | "Entity")
            || self.classes.contains_key(&named(ty))
        {
            if let Some(sel) = self.selector_of(init) {
                self.bindings.insert(name.to_string(), sel.emit());
            }
        }
        if let ExprKind::Call { callee, .. } = &init.kind {
            if let ExprKind::Field { name: m, base } = &callee.kind {
                if m == "of" {
                    if let Some(sel) = self.selector_of(base) {
                        self.bindings.insert(name.to_string(), sel.emit());
                    }
                }
            }
        }
    }

    fn consume_temps_in_stmt(&mut self, stmt: &Stmt) {
        let names: Vec<String> = self.temp_names.iter().cloned().collect();
        for n in names {
            if !stmt_uses_name(stmt, &n) {
                continue;
            }
            if !self.temp_consumed.insert(n.clone()) {
                self.errors.push(Diagnostic::new(
                    stmt.span.clone(),
                    format!("temp `{n}` was already consumed (one-shot; not a scoreboard)"),
                ));
            }
            self.locals_expr.remove(&n);
            self.consts.remove(&n);
            self.lists.remove(&n);
            self.bindings.remove(&n);
        }
    }

    fn int_of(&self, expr: &Expr) -> Option<i64> {
        match &expr.kind {
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let c = match &cond.kind {
                    ExprKind::Bool(b) => *b,
                    _ => self.int_of(cond)? != 0,
                };
                if c {
                    self.int_of(then_expr)
                } else {
                    self.int_of(else_expr)
                }
            }
            ExprKind::Ident(n) => {
                if let Some(ConstVal::Int(v)) = self.consts.get(n) {
                    return Some(*v);
                }
                if let Some(e) = self.locals_expr.get(n).cloned() {
                    return self.int_of(&e);
                }
                eval_int(expr, &self.enums)
            }
            _ => eval_int(expr, &self.enums),
        }
    }

    fn tag_of(&self, expr: &Expr) -> Option<String> {
        match &expr.kind {
            ExprKind::String(s) => Some(s.clone()),
            ExprKind::Ident(n) => {
                if let Some(ConstVal::Tag(t)) = self.consts.get(n) {
                    return Some(t.clone());
                }
                if let Some(e) = self.locals_expr.get(n).cloned() {
                    return self.tag_of(&e);
                }
                None
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    if name == "of" {
                        if let ExprKind::Ident(recv) = &base.kind {
                            if recv == "Tag" {
                                return args.first().and_then(string_lit);
                            }
                        }
                    }
                }
                None
            }
            ExprKind::Field { base, name } => {
                if let ExprKind::Ident(recv) = &base.kind {
                    if recv == "Tag" {
                        return Some(name.to_lowercase());
                    }
                }
                Some(name.clone())
            }
            _ => None,
        }
    }

    fn lower_tag_mut(&self, base: &Expr, verb: &str, args: &[Expr]) -> Vec<String> {
        let verb = if verb == "add" { "add" } else { "remove" };
        let (sel, tag) = if matches!(&base.kind, ExprKind::Ident(n) if n == "Tag")
            || self.tag_of(base).is_some()
                && self.selector_of(base).is_none()
                && !matches!(&base.kind, ExprKind::This)
        {
            let tag = self.tag_of(base).unwrap_or_default();
            let sel = args
                .first()
                .and_then(|e| self.selector_of(e))
                .map(|s| s.emit())
                .unwrap_or_else(|| self.this_sel.clone());
            (sel, tag)
        } else {
            let sel = self
                .selector_of(base)
                .map(|s| s.emit())
                .unwrap_or_else(|| self.this_sel.clone());
            let tag = args
                .first()
                .and_then(|e| self.tag_of(e))
                .unwrap_or_default();
            (sel, tag)
        };
        if tag.is_empty() {
            return Vec::new();
        }
        vec![format!("tag {sel} {verb} {tag}")]
    }

    fn lower_while(&mut self, cond: &Expr, body: &[Stmt]) -> Vec<String> {
        match &cond.kind {
            ExprKind::Bool(false) => Vec::new(),
            ExprKind::Bool(true) => {
                self.errors.push(Diagnostic::new(
                    cond.span.clone(),
                    "`while (true)` has no compile-time bound; use `for` or a clock `if`",
                ));
                Vec::new()
            }
            _ => {
                if let Some(n) = self.unroll_bound(cond) {
                    let mut out = Vec::new();
                    for _ in 0..n {
                        out.extend(self.lower_block(body));
                    }
                    return out;
                }
                let inner = self.lower_block(body);
                self.emit_if(cond, inner, Vec::new())
            }
        }
    }

    fn lower_for(
        &mut self,
        init: Option<&Stmt>,
        cond: Option<&Expr>,
        step: Option<&Expr>,
        body: &[Stmt],
    ) -> Vec<String> {
        if let Some(init) = init {
            let _ = self.lower_block(std::slice::from_ref(init));
        }
        if let (Some(cond), Some(step)) = (cond, step) {
            if let Some(times) = self.unroll_for_times(init, cond, step) {
                let mut out = Vec::new();
                for _ in 0..times {
                    out.extend(self.lower_block(body));
                    let _ = self.lower_expr_stmt(step);
                    self.apply_const_step(step);
                }
                return out;
            }
        }
        let mut out = Vec::new();
        if let Some(cond) = cond {
            let mut inner = self.lower_block(body);
            if let Some(step) = step {
                inner.extend(self.lower_expr_stmt(step));
            }
            out.extend(self.emit_if(cond, inner, Vec::new()));
        } else {
            out.extend(self.lower_block(body));
        }
        out
    }

    fn unroll_bound(&self, cond: &Expr) -> Option<u32> {
        if let ExprKind::Binary {
            op: BinOp::Lt | BinOp::Le,
            lhs,
            rhs,
        } = &cond.kind
        {
            let n = self.int_of(rhs)?;
            let start = self.int_of(lhs)?;
            let max = if matches!(cond.kind, ExprKind::Binary { op: BinOp::Le, .. }) {
                n - start + 1
            } else {
                n - start
            };
            if (1..65).contains(&max) {
                return Some(max as u32);
            }
        }
        None
    }

    fn unroll_for_times(&self, init: Option<&Stmt>, cond: &Expr, step: &Expr) -> Option<u32> {
        let start = match init.map(|s| &s.kind) {
            Some(StmtKind::Local { init: Some(e), .. }) => self.int_of(e)?,
            _ => 0,
        };
        let ExprKind::Binary {
            op: BinOp::Lt | BinOp::Le,
            rhs,
            ..
        } = &cond.kind
        else {
            return None;
        };
        let end = self.int_of(rhs)?;
        let delta = match &step.kind {
            ExprKind::Update { delta, .. } => *delta,
            _ => 1,
        };
        if delta <= 0 {
            return None;
        }
        let span = match cond.kind {
            ExprKind::Binary { op: BinOp::Le, .. } => end - start + 1,
            _ => end - start,
        };
        let times = span / delta;
        if (1..65).contains(&times) {
            Some(times as u32)
        } else {
            None
        }
    }

    fn apply_const_step(&mut self, step: &Expr) {
        if let ExprKind::Update { expr, delta, .. } = &step.kind {
            if let ExprKind::Ident(n) = &expr.kind {
                if let Some(ConstVal::Int(v)) = self.consts.get_mut(n) {
                    *v += *delta;
                }
            }
        }
    }

    fn list_elems(&self, expr: &Expr) -> Option<Vec<Expr>> {
        match &expr.kind {
            ExprKind::Ident(n) => self.lists.get(n).cloned(),
            ExprKind::Call { callee, args } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    if name == "of" {
                        if let ExprKind::Ident(recv) = &base.kind {
                            if recv == "List" {
                                return Some(args.clone());
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn is_item_expr(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Field { base, .. } => {
                matches!(&base.kind, ExprKind::Ident(n) if n == "Items")
            }
            ExprKind::Call { callee, .. } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    if matches!(
                        name.as_str(),
                        "count"
                            | "data"
                            | "component"
                            | "named"
                            | "name"
                            | "lore"
                            | "enchant"
                            | "trait"
                            | "glow"
                            | "lock"
                            | "tag"
                            | "withTag"
                    ) {
                        return self.is_item_expr(base);
                    }
                }
                false
            }
            ExprKind::Ident(n) => {
                if matches!(self.consts.get(n), Some(ConstVal::Item(_))) {
                    return true;
                }
                if let Some(e) = self.locals_expr.get(n).cloned() {
                    return self.is_item_expr(&e);
                }
                false
            }
            _ => false,
        }
    }

    fn item_of(&self, expr: &Expr) -> ItemStack {
        if let ExprKind::Ident(n) = &expr.kind {
            if let Some(ConstVal::Item(it)) = self.consts.get(n) {
                return it.clone();
            }
            if let Some(e) = self.locals_expr.get(n).cloned() {
                return self.item_of(&e);
            }
        }
        match &expr.kind {
            ExprKind::Call { callee, args } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    let mut item = self.item_of(base);
                    match name.as_str() {
                        "count" => {
                            if let Some(n) = args.first().and_then(|e| self.int_of(e)) {
                                item.count = n;
                            }
                        }
                        "data" => {
                            if let Some(n) = args.first().and_then(|e| self.int_of(e)) {
                                item.data = Some(n);
                            }
                        }
                        "component" => {
                            let k = args.first().and_then(string_lit).unwrap_or_default();
                            let v = args.get(1).and_then(string_lit).unwrap_or_default();
                            item.components.push((k, v));
                        }
                        "named" | "name" => {
                            if let Some(s) = args.first().and_then(string_lit) {
                                item.custom_name = Some(s);
                            }
                        }
                        "lore" => {
                            if let Some(s) = args.first().and_then(string_lit) {
                                item.lore.push(s);
                            }
                        }
                        "enchant" => {
                            let id = enum_or_name(args.first()).to_lowercase();
                            let lv = args.get(1).and_then(|e| self.int_of(e)).unwrap_or(1);
                            item.enchants.push((id, lv));
                        }
                        "trait" => {
                            if let Some(s) = args.first().and_then(string_lit) {
                                let sl = s.to_lowercase();
                                if sl == "lock" {
                                    item.lock = true;
                                }
                                if sl == "glow" {
                                    item.glow = true;
                                }
                                item.traits.push(s);
                            }
                        }
                        "glow" => item.glow = true,
                        "lock" => {
                            item.lock = true;
                            if !item.traits.iter().any(|t| t == "lock") {
                                item.traits.push("lock".into());
                            }
                        }
                        "tag" | "withTag" => {
                            if let Some(t) = args.first().and_then(|e| self.tag_of(e)) {
                                item.tags.push(t);
                            }
                        }
                        _ => {}
                    }
                    return item;
                }
                ItemStack::new("air")
            }
            ExprKind::Field { base, name } => {
                if let ExprKind::Ident(recv) = &base.kind {
                    if recv == "Items" {
                        return ItemStack::new(name);
                    }
                }
                ItemStack::new(name)
            }
            ExprKind::String(s) => ItemStack::new(s),
            _ => ItemStack::new(item_id(Some(expr))),
        }
    }

    fn block_of(&self, expr: &Expr) -> String {
        if let ExprKind::Ident(n) = &expr.kind {
            if let Some(e) = self.locals_expr.get(n).cloned() {
                return self.block_of(&e);
            }
        }
        match &expr.kind {
            ExprKind::Field { base, name } => {
                if let ExprKind::Ident(recv) = &base.kind {
                    if recv == "Blocks" {
                        return name.to_lowercase();
                    }
                }
                name.to_lowercase()
            }
            ExprKind::Ident(n) => n.to_lowercase(),
            ExprKind::String(s) => s.clone(),
            _ => block_id(expr),
        }
    }

    fn const_of(&self, expr: &Expr) -> Option<ConstVal> {
        if let ExprKind::Ident(n) = &expr.kind {
            if let Some(v) = self.consts.get(n) {
                return Some(v.clone());
            }
            if let Some(e) = self.locals_expr.get(n).cloned() {
                return self.const_of(&e);
            }
        }
        if self.is_item_expr(expr) {
            return Some(ConstVal::Item(self.item_of(expr)));
        }
        if let ExprKind::Call { callee, args } = &expr.kind {
            if let ExprKind::Field { base, name } = &callee.kind {
                if name == "of" {
                    if let ExprKind::Ident(recv) = &base.kind {
                        if recv == "List" {
                            let xs: Option<Vec<ConstVal>> =
                                args.iter().map(|a| self.const_of(a)).collect();
                            return xs.map(ConstVal::List);
                        }
                        if recv == "Tag" {
                            return args.first().and_then(string_lit).map(ConstVal::Tag);
                        }
                    }
                }
            }
        }
        if let Some(pos) = self.pos_of(expr) {
            return Some(ConstVal::Pos(pos));
        }
        eval_const(expr)
    }

    fn lvalue(&self, expr: &Expr) -> Option<(String, FieldMeta)> {
        match &expr.kind {
            ExprKind::Field { base, name } => {
                let owner = match &base.kind {
                    ExprKind::This => self.current_class.clone(),
                    ExprKind::Ident(n) if self.classes.contains_key(n) => n.clone(),
                    ExprKind::Ident(n) => {
                        if let Some(ty) = self.bindings.get(n) {
                            let _ = ty;
                        }
                        self.current_class.clone()
                    }
                    _ => self.call_owner(base)?,
                };
                let mut owner = owner;
                if let ExprKind::Ident(n) = &base.kind {
                    if self.classes.contains_key(n) {
                        owner = n.clone();
                    }
                }
                let field = self.fields.get(&(owner, name.clone()))?;
                let holder = if field.is_static {
                    self.config.edition.world_holder().to_string()
                } else {
                    match &base.kind {
                        ExprKind::This => self.this_sel.clone(),
                        ExprKind::Ident(n) => self
                            .bindings
                            .get(n)
                            .cloned()
                            .unwrap_or_else(|| self.this_sel.clone()),
                        _ => self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| self.this_sel.clone()),
                    }
                };
                Some((holder, field.clone()))
            }
            _ => None,
        }
    }

    fn selector_of(&self, expr: &Expr) -> Option<Selector> {
        match &expr.kind {
            ExprKind::Ident(name) => {
                if let Some(sel) = self.bindings.get(name) {
                    return Some(Selector::simple(sel));
                }
                if let Some(e) = self.locals_expr.get(name).cloned() {
                    return self.selector_of(&e);
                }
                None
            }
            ExprKind::This => Some(Selector::simple(&self.this_sel)),
            ExprKind::Call { callee, args } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    if let ExprKind::Ident(recv) = &base.kind {
                        match (recv.as_str(), name.as_str()) {
                            ("Player", "self") | ("Players", "self") => {
                                return Some(Selector::simple("@s"));
                            }
                            ("Player", "nearest") => return Some(Selector::simple("@p")),
                            ("Players", "all") => return Some(Selector::simple("@a")),
                            ("Players", "entities") => return Some(Selector::simple("@e")),
                            ("Players", "raw") => {
                                return Some(Selector::simple(&string_lit(&args[0])?));
                            }
                            _ => {}
                        }
                    }
                    if name == "of" {
                        return args.first().and_then(|a| self.selector_of(a));
                    }
                    if name == "exists" {
                        return self.selector_of(base);
                    }
                    let mut sel = self.selector_of(base)?;
                    self.apply_sel(&mut sel, name, args)?;
                    return Some(sel);
                }
                None
            }
            _ => None,
        }
    }

    fn apply_sel(&self, sel: &mut Selector, name: &str, args: &[Expr]) -> Option<()> {
        match name {
            "withTag" => {
                let t = args.first().and_then(|e| self.tag_of(e))?;
                sel.args.push(("tag".into(), t));
            }
            "withoutTag" => {
                let t = args.first().and_then(|e| self.tag_of(e))?;
                sel.args.push(("tag".into(), format!("!{t}")));
            }
            "inBox" => {
                let nums: Vec<i64> = args.iter().filter_map(|e| self.int_of(e)).collect();
                if nums.len() == 6 {
                    sel.args.push(("x".into(), nums[0].to_string()));
                    sel.args.push(("y".into(), nums[1].to_string()));
                    sel.args.push(("z".into(), nums[2].to_string()));
                    sel.args.push(("dx".into(), nums[3].to_string()));
                    sel.args.push(("dy".into(), nums[4].to_string()));
                    sel.args.push(("dz".into(), nums[5].to_string()));
                }
            }
            "within" => {
                let r = self.int_of(args.get(1)?)?;
                match self.config.edition {
                    Edition::Bedrock => sel.args.push(("r".into(), r.to_string())),
                    Edition::Java => sel.args.push(("distance".into(), format!("..{r}"))),
                }
            }
            "in" => {
                let region = self.region_of(args.first()?)?;
                sel.args.push(("x".into(), region.0.to_string()));
                sel.args.push(("y".into(), region.1.to_string()));
                sel.args.push(("z".into(), region.2.to_string()));
                sel.args.push(("dx".into(), region.3.to_string()));
                sel.args.push(("dy".into(), region.4.to_string()));
                sel.args.push(("dz".into(), region.5.to_string()));
            }
            "hasItem" => {
                let item = args.first().map(|e| self.item_of(e))?;
                let id = item.id.clone();
                let n = args
                    .get(1)
                    .and_then(|e| self.int_of(e))
                    .unwrap_or(item.count.max(1));
                match self.config.edition {
                    Edition::Bedrock => sel
                        .args
                        .push(("hasitem".into(), format!("{{item={id},quantity={n}..}}"))),
                    Edition::Java => {
                        let item = if id.contains(':') {
                            id
                        } else {
                            format!("minecraft:{id}")
                        };
                        sel.java_items.push((item, n));
                    }
                }
            }
            _ => return None,
        }
        Some(())
    }

    fn region_of(&self, expr: &Expr) -> Option<(i64, i64, i64, i64, i64, i64)> {
        if let Some(ConstVal::Region {
            x,
            y,
            z,
            dx,
            dy,
            dz,
        }) = eval_const(expr)
        {
            return Some((x, y, z, dx, dy, dz));
        }
        if let ExprKind::Field { base, name } = &expr.kind {
            if let ExprKind::Ident(c) = &base.kind {
                if let Some(ConstVal::Region {
                    x,
                    y,
                    z,
                    dx,
                    dy,
                    dz,
                }) = self.consts.get(&format!("{c}.{name}"))
                {
                    return Some((*x, *y, *z, *dx, *dy, *dz));
                }
            }
        }
        if let ExprKind::Ident(n) = &expr.kind {
            if let Some(ConstVal::Region {
                x,
                y,
                z,
                dx,
                dy,
                dz,
            }) = self.consts.get(n)
            {
                return Some((*x, *y, *z, *dx, *dy, *dz));
            }
        }
        None
    }

    fn pos_of(&self, expr: &Expr) -> Option<String> {
        if let ExprKind::Ident(n) = &expr.kind {
            if let Some(ConstVal::Pos(p)) = self.consts.get(n) {
                return Some(p.clone());
            }
            if let Some(e) = self.locals_expr.get(n).cloned() {
                return self.pos_of(&e);
            }
        }
        fn walk(expr: &Expr, x: &mut String, y: &mut String, z: &mut String) -> bool {
            match &expr.kind {
                ExprKind::Call { callee, args } => {
                    if let ExprKind::Field { base, name } = &callee.kind {
                        if let ExprKind::Ident(recv) = &base.kind {
                            if recv == "BlockPos" && name == "here" {
                                *x = "~".into();
                                *y = "~".into();
                                *z = "~".into();
                                return true;
                            }
                            if recv == "BlockPos" && name == "of" && args.len() == 3 {
                                *x = lit_coord(&args[0]);
                                *y = lit_coord(&args[1]);
                                *z = lit_coord(&args[2]);
                                return true;
                            }
                        }
                        if walk(base, x, y, z) {
                            let n = args.first().and_then(expr_int).unwrap_or(1);
                            match name.as_str() {
                                "up" => *y = offset_coord(y, n),
                                "down" => *y = offset_coord(y, -n),
                                "east" => *x = offset_coord(x, n),
                                "west" => *x = offset_coord(x, -n),
                                "south" => *z = offset_coord(z, n),
                                "north" => *z = offset_coord(z, -n),
                                _ => {}
                            }
                            return true;
                        }
                    }
                    false
                }
                _ => false,
            }
        }
        let mut x = "0".into();
        let mut y = "0".into();
        let mut z = "0".into();
        if walk(expr, &mut x, &mut y, &mut z) {
            Some(format!("{x} {y} {z}"))
        } else if let Some([a, b, c]) = triple(expr) {
            Some(format!("{a} {b} {c}"))
        } else {
            None
        }
    }

    fn emit_if(&self, cond: &Expr, then_cmds: Vec<String>, else_cmds: Vec<String>) -> Vec<String> {
        match &cond.kind {
            ExprKind::Bool(true) => return then_cmds,
            ExprKind::Bool(false) => return else_cmds,
            _ => {}
        }
        let mut out = Vec::new();
        let or_parts = flatten_or(cond);
        for part in &or_parts {
            let yes = self.and_clauses(part, false);
            out.extend(wrap_clauses(&yes, then_cmds.clone()));
        }
        if !else_cmds.is_empty() {
            if or_parts.len() > 1 {
                let mut no = Vec::new();
                for part in &or_parts {
                    no.extend(self.and_clauses(part, true));
                }
                out.extend(wrap_clauses(&no, else_cmds));
            } else if let ExprKind::Binary {
                op: BinOp::And,
                lhs,
                rhs,
            } = &cond.kind
            {
                out.extend(wrap_clauses(
                    &self.and_clauses(lhs, true),
                    else_cmds.clone(),
                ));
                out.extend(wrap_clauses(&self.and_clauses(rhs, true), else_cmds));
            } else {
                let no = self.and_clauses(cond, true);
                out.extend(wrap_clauses(&no, else_cmds));
            }
        }
        out
    }

    fn and_clauses(&self, expr: &Expr, invert: bool) -> Vec<String> {
        match &expr.kind {
            ExprKind::Binary {
                op: BinOp::And,
                lhs,
                rhs,
            } if !invert => {
                let mut a = self.and_clauses(lhs, false);
                a.extend(self.and_clauses(rhs, false));
                a
            }
            _ => self.cond_clauses(expr, invert),
        }
    }

    fn java_item_clauses(&self, sel: &Selector) -> Vec<String> {
        if self.config.edition != Edition::Java {
            return Vec::new();
        }
        sel.java_items
            .iter()
            .map(|(item, n)| format!("if items entity @s container.* {item} {n}.."))
            .collect()
    }

    fn cond_clauses(&self, expr: &Expr, invert: bool) -> Vec<String> {
        match &expr.kind {
            ExprKind::Binary {
                op: BinOp::And,
                lhs,
                rhs,
            } => {
                let mut a = self.cond_clauses(lhs, invert);
                if invert {
                    return a;
                }
                a.extend(self.cond_clauses(rhs, false));
                a
            }
            ExprKind::Binary {
                op: BinOp::Or,
                lhs,
                rhs,
            } => {
                if invert {
                    let mut a = self.cond_clauses(lhs, true);
                    a.extend(self.cond_clauses(rhs, true));
                    a
                } else {
                    self.cond_clauses(lhs, false)
                }
            }
            ExprKind::Unary {
                op: UnaryOp::Not,
                expr,
            } => self.cond_clauses(expr, !invert),
            ExprKind::Binary { op, lhs, rhs } if is_cmp(*op) => {
                if let Some(block) = self.block_eq(lhs, rhs, *op) {
                    let word = if invert { "unless" } else { "if" };
                    return vec![format!("{word} block {block}")];
                }
                if let Some(cl) = self.score_cmp(lhs, rhs, *op, invert) {
                    return vec![cl];
                }
                Vec::new()
            }
            ExprKind::Field { .. } => {
                if let Some((holder, field)) = self.lvalue(expr) {
                    if field.kind == SymbolKind::Tag {
                        let word = if invert { "unless" } else { "if" };
                        return vec![format!("{word} entity {holder}[tag={}]", field.short)];
                    }
                    if field.kind == SymbolKind::Objective {
                        let word = if invert { "unless" } else { "if" };
                        return vec![format!("{word} score {holder} {} matches 1..", field.short)];
                    }
                }
                Vec::new()
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::Field { base, name } = &callee.kind {
                    if name == "exists" {
                        if let Some(sel) = self.selector_of(base) {
                            let word = if invert { "unless" } else { "if" };
                            let mut clauses = vec![format!("{word} entity {}", sel.emit())];
                            if !invert {
                                clauses.extend(self.java_item_clauses(&sel));
                            }
                            return clauses;
                        }
                    }
                    if name == "has" || name == "hasTag" {
                        if let Some(tag) = args.first().and_then(|e| self.tag_of(e)) {
                            let sel = self
                                .selector_of(base)
                                .map(|s| s.emit())
                                .unwrap_or_else(|| self.this_sel.clone());
                            let word = if invert { "unless" } else { "if" };
                            return vec![format!("{word} entity {sel}[tag={tag}]")];
                        }
                    }
                    if name == "in" {
                        if let Some(mut sel) = self.selector_of(base) {
                            if self.apply_sel(&mut sel, "in", args).is_some() {
                                let word = if invert { "unless" } else { "if" };
                                return vec![format!("{word} entity {}", sel.emit())];
                            }
                        }
                    }
                }
                if let ExprKind::Ident(n) = &callee.kind {
                    if n == "block" {
                        if let Some(pos) = args.first().and_then(|e| self.pos_of(e)) {
                            let word = if invert { "unless" } else { "if" };
                            return vec![format!("{word} block {pos} air")];
                        }
                    }
                }
                Vec::new()
            }
            ExprKind::Bool(true) if !invert => Vec::new(),
            ExprKind::Bool(false) => vec!["if entity @s[tag=__never]".into()],
            _ => Vec::new(),
        }
    }

    fn score_cmp(&self, lhs: &Expr, rhs: &Expr, op: BinOp, invert: bool) -> Option<String> {
        let (holder, field) = self.lvalue(lhs)?;
        if field.kind != SymbolKind::Objective {
            return None;
        }
        let n = self.int_of(rhs)?;
        let range = match op {
            BinOp::Eq => format!("{n}"),
            BinOp::Ne => format!("{n}"),
            BinOp::Ge => format!("{n}.."),
            BinOp::Gt => format!("{}..", n + 1),
            BinOp::Le => format!("..{n}"),
            BinOp::Lt => format!("..{}", n - 1),
            BinOp::In => {
                if let ExprKind::Binary {
                    op: BinOp::Range,
                    lhs,
                    rhs,
                } = &rhs.kind
                {
                    let a = self.int_of(lhs)?;
                    let b = self.int_of(rhs)?;
                    format!("{a}..{b}")
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        let mut word_if = !invert;
        if op == BinOp::Ne {
            word_if = invert;
        }
        let word = if word_if { "if" } else { "unless" };
        Some(format!(
            "{word} score {holder} {} matches {range}",
            field.short
        ))
    }

    fn block_eq(&self, lhs: &Expr, rhs: &Expr, op: BinOp) -> Option<String> {
        if op != BinOp::Eq && op != BinOp::Ne {
            return None;
        }
        let ExprKind::Call { callee, args } = &lhs.kind else {
            return None;
        };
        let ExprKind::Ident(n) = &callee.kind else {
            return None;
        };
        if n != "block" {
            return None;
        }
        let pos = self.pos_of(args.first()?)?;
        let id = block_id(rhs);
        Some(format!("{pos} {id}"))
    }
}

fn enum_or_name_expr(expr: &Expr) -> String {
    enum_or_name(Some(expr))
}

fn stmt_uses_name(stmt: &Stmt, name: &str) -> bool {
    match &stmt.kind {
        StmtKind::Annotated { inner, .. } => stmt_uses_name(inner, name),
        StmtKind::Local { init, .. } => init.as_ref().is_some_and(|e| expr_uses_name(e, name)),
        StmtKind::If {
            cond,
            then_body,
            else_body,
        } => {
            expr_uses_name(cond, name)
                || then_body.iter().any(|s| stmt_uses_name(s, name))
                || else_body
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| stmt_uses_name(s, name)))
        }
        StmtKind::Foreach { iter, body, .. } => {
            expr_uses_name(iter, name) || body.iter().any(|s| stmt_uses_name(s, name))
        }
        StmtKind::Switch {
            expr,
            arms,
            default,
        } => {
            expr_uses_name(expr, name)
                || arms.iter().any(|(p, b)| {
                    expr_uses_name(p, name) || b.iter().any(|s| stmt_uses_name(s, name))
                })
                || default
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| stmt_uses_name(s, name)))
        }
        StmtKind::Context { prefixes, body } => {
            prefixes.iter().any(|p| match p {
                ContextPrefix::As(e) | ContextPrefix::At(e) | ContextPrefix::Facing(e) => {
                    expr_uses_name(e, name)
                }
                _ => false,
            }) || body.iter().any(|s| stmt_uses_name(s, name))
        }
        StmtKind::Return(Some(expr)) | StmtKind::Expr(expr) => expr_uses_name(expr, name),
        StmtKind::Assign { target, value, .. } => {
            expr_uses_name(target, name) || expr_uses_name(value, name)
        }
        StmtKind::While { cond, body } => {
            expr_uses_name(cond, name) || body.iter().any(|s| stmt_uses_name(s, name))
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            init.as_ref().is_some_and(|s| stmt_uses_name(s, name))
                || cond.as_ref().is_some_and(|e| expr_uses_name(e, name))
                || step.as_ref().is_some_and(|e| expr_uses_name(e, name))
                || body.iter().any(|s| stmt_uses_name(s, name))
        }
        StmtKind::Run { command } => command
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == name),
        StmtKind::Return(None) | StmtKind::Label(_) => false,
    }
}

fn expr_uses_name(expr: &Expr, name: &str) -> bool {
    match &expr.kind {
        ExprKind::Ident(n) => n == name,
        ExprKind::Unary { expr, .. } => expr_uses_name(expr, name),
        ExprKind::Binary { lhs, rhs, .. } => expr_uses_name(lhs, name) || expr_uses_name(rhs, name),
        ExprKind::Call { callee, args } => {
            expr_uses_name(callee, name) || args.iter().any(|a| expr_uses_name(a, name))
        }
        ExprKind::Field { base, .. } => expr_uses_name(base, name),
        ExprKind::New { args, .. } => args.iter().any(|a| expr_uses_name(a, name)),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_uses_name(cond, name)
                || expr_uses_name(then_expr, name)
                || expr_uses_name(else_expr, name)
        }
        ExprKind::Update { expr, .. } => expr_uses_name(expr, name),
        _ => false,
    }
}

fn peel_stmt_meta(stmt: &Stmt) -> (Stmt, ChainCmdMeta) {
    let mut meta = ChainCmdMeta::default();
    let mut cur = stmt;
    loop {
        match &cur.kind {
            StmtKind::Annotated { annotations, inner } => {
                for a in annotations {
                    match a.name.as_str() {
                        "Delay" => {
                            if let Some(arg) = a.args.first() {
                                if let ExprKind::Int(n) = &arg.kind {
                                    meta.delay = *n as u32;
                                }
                            }
                        }
                        "Conditional" => meta.conditional = true,
                        "Impulse" => meta.impulse = true,
                        "Repeat" => meta.repeat = true,
                        "AlwaysActive" => meta.always_active = true,
                        "NeedsRedstone" => meta.always_active = false,
                        _ => {}
                    }
                }
                cur = inner;
            }
            StmtKind::Context { prefixes, body } => {
                for p in prefixes {
                    if let ContextPrefix::At(e) = p {
                        if let Some(coords) = triple_ints(e) {
                            meta.at = Some(coords);
                        }
                    }
                }
                if body.len() == 1 {
                    let (inner, nested) = peel_stmt_meta(&body[0]);
                    if nested.delay != 0 {
                        meta.delay = nested.delay;
                    }
                    if nested.conditional {
                        meta.conditional = true;
                    }
                    if nested.at.is_some() {
                        meta.at = nested.at;
                    }
                    return (inner, meta);
                }
                return (cur.clone(), meta);
            }
            _ => return (cur.clone(), meta),
        }
    }
}

fn triple_ints(expr: &Expr) -> Option<[i32; 3]> {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let ExprKind::Field { name, .. } = &callee.kind {
                if name == "of" && args.len() == 3 {
                    return Some([
                        expr_int(&args[0])? as i32,
                        expr_int(&args[1])? as i32,
                        expr_int(&args[2])? as i32,
                    ]);
                }
            }
            None
        }
        _ => None,
    }
}

fn flatten_or(expr: &Expr) -> Vec<&Expr> {
    match &expr.kind {
        ExprKind::Binary {
            op: BinOp::Or,
            lhs,
            rhs,
        } => {
            let mut v = flatten_or(lhs);
            v.extend(flatten_or(rhs));
            v
        }
        _ => vec![expr],
    }
}

fn wrap_clauses(clauses: &[String], cmds: Vec<String>) -> Vec<String> {
    if clauses.is_empty() {
        return cmds;
    }
    let prefix = format!("execute {}", clauses.join(" "));
    cmds.into_iter()
        .map(|c| {
            if c.starts_with("execute ") {
                format!("{prefix} {}", c.trim_start_matches("execute "))
            } else {
                format!("{prefix} run {c}")
            }
        })
        .collect()
}

fn merge_as_at(cmds: &[String]) -> Vec<String> {
    cmds.to_vec()
}

fn is_bare_return(body: &[Stmt]) -> bool {
    matches!(
        body,
        [Stmt {
            kind: StmtKind::Return(None),
            ..
        }]
    )
}

fn is_cmp(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::In
    )
}

fn is_arith(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem
    )
}

fn is_score_arith(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Binary { op, .. } if is_arith(*op))
}

fn bin_mop(op: BinOp) -> Option<&'static str> {
    Some(match op {
        BinOp::Add => "+=",
        BinOp::Sub => "-=",
        BinOp::Mul => "*=",
        BinOp::Div => "/=",
        BinOp::Rem => "%=",
        _ => return None,
    })
}

fn assign_mop(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Eq => "=",
        AssignOp::PlusEq => "+=",
        AssignOp::MinusEq => "-=",
        AssignOp::StarEq => "*=",
        AssignOp::SlashEq => "/=",
        AssignOp::PercentEq => "%=",
    }
}

fn named(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Named(p) => p.parts.last().cloned().unwrap_or_default(),
        TypeRef::Seq(inner) => named(inner),
        TypeRef::List(_) => "List".into(),
        TypeRef::Int => "int".into(),
        TypeRef::Boolean => "boolean".into(),
        TypeRef::Void => "void".into(),
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::String(s) => Some(s.clone()),
        ExprKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => Some(format!("{}{}", string_lit(lhs)?, string_lit(rhs)?)),
        _ => None,
    }
}

fn eval_int(expr: &Expr, enums: &HashMap<String, Vec<String>>) -> Option<i64> {
    match &expr.kind {
        ExprKind::Int(n) => Some(*n),
        ExprKind::Bool(true) => Some(1),
        ExprKind::Bool(false) => Some(0),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            expr,
        } => Some(-eval_int(expr, enums)?),
        ExprKind::Field { base, name } => {
            if let ExprKind::Ident(en) = &base.kind {
                if let Some(vars) = enums.get(en) {
                    return vars.iter().position(|v| v == name).map(|i| i as i64);
                }
            }
            None
        }
        _ => expr_int(expr),
    }
}

fn eval_const(expr: &Expr) -> Option<ConstVal> {
    match &expr.kind {
        ExprKind::Int(n) => Some(ConstVal::Int(*n)),
        ExprKind::Bool(b) => Some(ConstVal::Bool(*b)),
        ExprKind::String(s) => Some(ConstVal::Str(s.clone())),
        ExprKind::Call { callee, args } => {
            if let ExprKind::Field { base, name } = &callee.kind {
                if let ExprKind::Ident(recv) = &base.kind {
                    if recv == "Region" && name == "box" && args.len() == 6 {
                        return Some(ConstVal::Region {
                            x: expr_int(&args[0])?,
                            y: expr_int(&args[1])?,
                            z: expr_int(&args[2])?,
                            dx: expr_int(&args[3])?,
                            dy: expr_int(&args[4])?,
                            dz: expr_int(&args[5])?,
                        });
                    }
                    if recv == "Tag" && name == "of" {
                        return args.first().and_then(string_lit).map(ConstVal::Tag);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn render_lit(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Int(n) => n.to_string(),
        ExprKind::String(s) => s.clone(),
        _ => String::new(),
    }
}

fn enum_or_name(expr: Option<&Expr>) -> String {
    match expr.map(|e| &e.kind) {
        Some(ExprKind::Field { name, .. }) => name.clone(),
        Some(ExprKind::Ident(n)) => n.clone(),
        Some(ExprKind::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn item_id(expr: Option<&Expr>) -> String {
    match expr.map(|e| &e.kind) {
        Some(ExprKind::Field { name, .. }) => name.to_lowercase(),
        Some(ExprKind::Ident(n)) => n.to_lowercase(),
        Some(ExprKind::String(s)) => s.clone(),
        _ => "air".into(),
    }
}

fn block_id(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Field { name, .. } => name.to_lowercase(),
        ExprKind::Ident(n) => n.to_lowercase(),
        _ => "air".into(),
    }
}

fn lit_coord(expr: &Expr) -> String {
    expr_int(expr)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "~".into())
}

fn offset_coord(cur: &str, n: i64) -> String {
    if cur == "~" {
        if n == 0 {
            "~".into()
        } else {
            format!("~{n:+}")
        }
    } else if let Ok(v) = cur.parse::<i64>() {
        (v + n).to_string()
    } else {
        cur.to_string()
    }
}

fn triple(expr: &Expr) -> Option<[String; 3]> {
    let _ = expr;
    None
}

pub fn dump_commands(lowered: &Lowered) -> String {
    let mut out = String::new();
    for f in &lowered.functions {
        out.push_str(&format!("# function {}\n", f.path));
        for c in &f.commands {
            out.push_str(c);
            out.push('\n');
        }
    }
    for ch in &lowered.chains {
        out.push_str(&format!("# chain {}\n", ch.name));
        for (c, _) in &ch.commands {
            out.push_str(c);
            out.push('\n');
        }
    }
    out
}

pub fn resolve_chain_origin(
    info: &BinaryInfo,
    config: &MincConfig,
    chain: &str,
) -> ([i32; 3], Facing, String) {
    let world = config.origin;
    if let Some(c) = info.chains.iter().find(|c| c.name == chain) {
        let rel = c.origin.unwrap_or([0, 0, 0]);
        let abs = if c.absolute {
            [rel[0] as i32, rel[1] as i32, rel[2] as i32]
        } else {
            [
                world[0] + rel[0] as i32,
                world[1] + rel[1] as i32,
                world[2] + rel[2] as i32,
            ]
        };
        let facing = Facing::parse(c.facing.as_deref().unwrap_or(&config.default_facing))
            .unwrap_or(Facing::Up);
        let layout = c
            .layout
            .clone()
            .unwrap_or_else(|| config.default_layout.clone());
        (abs, facing, layout)
    } else {
        (
            world,
            Facing::parse(&config.default_facing).unwrap_or(Facing::Up),
            config.default_layout.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract_binary_info;

    #[test]
    fn lowers_static_score_add() {
        let src = r#"
pack metro.escape;
public class Match {
    public static int playTick;
}
@Repeat
public chain Play {
    Match.playTick += 1;
}
"#;
        let unit = crate::parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = MincConfig {
            edition: Edition::Bedrock,
            pack: "metro.escape".into(),
            ..MincConfig::default()
        };
        let lowered = lower_project(&unit, &cfg, &info).unwrap();
        let dump = dump_commands(&lowered);
        assert!(dump.contains("scoreboard players add mcs"), "{dump}");
        assert!(
            dump.contains("playTick") || dump.contains("m00") || dump.contains("m0"),
            "{dump}"
        );
    }

    #[test]
    fn java_within_uses_distance() {
        let src = r#"
pack demo;
public chain C {
    foreach (Player p : Players.all().within(BlockPos.of(0, 64, 0), 4)) {
        as (p) { cmd("say hi"); }
    }
}
"#;
        let unit = crate::parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = MincConfig {
            edition: Edition::Java,
            pack: "demo".into(),
            game_version: "1.21.11".into(),
            ..MincConfig::default()
        };
        let dump = dump_commands(&lower_project(&unit, &cfg, &info).unwrap());
        assert!(dump.contains("distance=..4"), "{dump}");
        assert!(!dump.contains("r=4"), "{dump}");
    }
}
