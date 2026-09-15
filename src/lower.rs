//! Lower MincScript to edition-specific Minecraft commands (no leading `/`).
//!
//! Bedrock and Java each get their own execute/selector printer. They do not
//! share an execute walker (`docs/minecraft-commands/crosswalk/parser-implementation.md`).

use std::collections::HashMap;

use crate::ast::{
    AssignOp, BinOp, ChainDef, ClassDef, CompilationUnit, ContextPrefix, Expr, Item, Member,
    MethodDef, Stmt, TypeRef, UnaryOp,
};
use crate::config::{Edition, MincConfig};
use crate::diagnostic::Diagnostic;
use crate::extract::{expr_int, BinaryInfo, SymbolKind};
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
    errors: Vec<Diagnostic>,
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
            errors: Vec::new(),
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
        setup.push(format!(
            "scoreboard players add {} mt 0",
            self.config.edition.world_holder()
        ));
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
                        for p in &method.params {
                            if matches!(named(&p.ty).as_str(), "Player" | "Entity" | "Runner")
                                || self.classes.contains_key(&named(&p.ty))
                            {
                                self.bindings.insert(p.name.clone(), "@s".into());
                            }
                        }
                        let mut cmds = self.lower_block(&method.body);
                        if cmds.len() as u32 > self.config.function_command_limit {
                            self.errors.push(Diagnostic::new(
                                0..0,
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
                let commands = self.lower_chain(chain);
                chains.push(LoweredChain {
                    name: chain.name.clone(),
                    commands,
                });
            }
        }

        Lowered {
            functions,
            chains,
            setup,
            tick_paths,
            load_paths,
        }
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
            match inner {
                Stmt::Label(name) => {
                    pending_label = Some(name.clone());
                    for cmd in self.link_cmds(&chain.name, &name) {
                        let mut m = meta.clone();
                        m.label = Some(name.clone());
                        out.push((cmd, m));
                    }
                }
                other => {
                    let cmds = match other {
                        Stmt::Return(_) | Stmt::Local { .. } => self.lower_block(&[other.clone()]),
                        _ => self.lower_stmt(&other),
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
            if let Stmt::If {
                cond,
                then_body,
                else_body: None,
            } = &stmts[i]
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
            match &stmts[i] {
                Stmt::Annotated { inner, annotations } => {
                    let _ = annotations;
                    out.extend(self.lower_block(std::slice::from_ref(inner.as_ref())));
                }
                Stmt::Local { name, init, ty } => {
                    if let Some(init) = init {
                        if matches!(named(ty).as_str(), "Player" | "Entity")
                            || self.classes.contains_key(&named(ty))
                        {
                            if let Some(sel) = self.selector_of(init) {
                                self.bindings.insert(name.clone(), sel.emit());
                            }
                        }
                        if let Expr::Call { callee, .. } = init {
                            if let Expr::Field { name: m, base } = callee.as_ref() {
                                if m == "of" {
                                    if let Some(sel) = self.selector_of(base) {
                                        self.bindings.insert(name.clone(), sel.emit());
                                    }
                                }
                            }
                        }
                    }
                }
                Stmt::Label(_) => {}
                Stmt::Return(_) => {
                    if self.config.edition == Edition::Java {
                        out.push("return".into());
                    }
                    break;
                }
                other => out.extend(self.lower_stmt(other)),
            }
            i += 1;
        }
        out
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> Vec<String> {
        match stmt {
            Stmt::Annotated { inner, .. } => self.lower_stmt(inner),
            Stmt::If {
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
            Stmt::Foreach {
                name, iter, body, ..
            } => {
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
            Stmt::Context { prefixes, body } => {
                let mut clauses = Vec::new();
                for p in prefixes {
                    match p {
                        ContextPrefix::As(e) => {
                            let sel = self
                                .selector_of(e)
                                .map(|s| s.emit())
                                .unwrap_or_else(|| "@s".into());
                            if let Expr::Ident(n) = e {
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
            Stmt::Switch {
                expr,
                arms,
                default,
            } => {
                let mut out = Vec::new();
                for (pat, body) in arms {
                    let cmp = Expr::Binary {
                        op: BinOp::Eq,
                        lhs: Box::new(expr.clone()),
                        rhs: Box::new(pat.clone()),
                    };
                    let cmds = self.lower_block(body);
                    let cl = self.cond_clauses(&cmp, false);
                    out.extend(wrap_clauses(&cl, cmds));
                }
                if let Some(body) = default {
                    out.extend(self.lower_block(body));
                }
                out
            }
            Stmt::Assign { target, op, value } => self.lower_assign(target, *op, value),
            Stmt::Expr(expr) => self.lower_expr_stmt(expr),
            Stmt::Return(_) | Stmt::Label(_) | Stmt::Local { .. } => Vec::new(),
        }
    }

    fn lower_assign(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> Vec<String> {
        if let Some((holder, field)) = self.lvalue(target) {
            match field.kind {
                SymbolKind::Tag => {
                    let add = match value {
                        Expr::Bool(true) => true,
                        Expr::Bool(false) => false,
                        _ => true,
                    };
                    let verb = if add { "add" } else { "remove" };
                    return vec![format!("tag {holder} {verb} {}", field.short)];
                }
                SymbolKind::Objective => {
                    if let Some(n) = eval_int(value, &self.enums) {
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
        if let Some(n) = eval_int(value, &self.enums) {
            out.push(format!("scoreboard players set #t0 mt {n}"));
            let mop = match op {
                AssignOp::Eq => "=",
                AssignOp::PlusEq => "+=",
                AssignOp::MinusEq => "-=",
                AssignOp::StarEq => "*=",
                AssignOp::SlashEq => "/=",
                AssignOp::PercentEq => "%=",
            };
            out.push(format!(
                "scoreboard players operation {holder} {obj} {mop} #t0 mt"
            ));
        }
        out
    }

    fn lower_expr_stmt(&mut self, expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Call { callee, args } => {
                if let Expr::Ident(name) = callee.as_ref() {
                    return self.lower_builtin_call(name, args);
                }
                if let Expr::Field { base, name } = callee.as_ref() {
                    if name == "of" {
                        return Vec::new();
                    }
                    if let Expr::Ident(recv) = base.as_ref() {
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
                        return vec![format!("gamemode {mode} {sel}")];
                    }
                    if name == "give" {
                        let sel = self
                            .selector_of(base)
                            .map(|s| s.emit())
                            .unwrap_or_else(|| "@s".into());
                        let item = item_id(args.first());
                        let count = args
                            .get(1)
                            .and_then(|e| eval_int(e, &self.enums))
                            .unwrap_or(1);
                        return vec![format!("give {sel} {item} {count}")];
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
                        return vec![format!("teleport {sel} {pos}")];
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
                if let Expr::Field { base, name } = a {
                    if let Expr::Ident(c) = base.as_ref() {
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
        match expr {
            Expr::Ident(n) if self.classes.contains_key(n) => Some(n.clone()),
            Expr::Call { callee, .. } => {
                if let Expr::Field { base, name } = callee.as_ref() {
                    if name == "of" {
                        if let Expr::Ident(c) = base.as_ref() {
                            return Some(c.clone());
                        }
                    }
                }
                self.call_owner(callee)
            }
            Expr::Field { base, .. } => self.call_owner(base),
            Expr::This => Some(self.current_class.clone()),
            _ => None,
        }
    }

    fn lower_builtin_call(&mut self, name: &str, args: &[Expr]) -> Vec<String> {
        match name {
            "cmd" => {
                let mut s = string_lit(&args[0]).unwrap_or_default();
                if let Some(rest) = s.strip_prefix('/') {
                    s = rest.to_string();
                }
                vec![s]
            }
            "title" => {
                let sel = args
                    .first()
                    .and_then(|e| self.selector_of(e))
                    .map(|s| s.emit())
                    .unwrap_or_else(|| "@a".into());
                let loc = enum_or_name(args.get(1)).to_lowercase();
                let loc = match loc.as_str() {
                    "title" | "subtitle" | "actionbar" | "times" | "clear" | "reset" => loc,
                    _ => "title".into(),
                };
                let text = args.get(2).and_then(string_lit).unwrap_or_default();
                match self.config.edition {
                    Edition::Bedrock => vec![format!("title {sel} {loc} {text}")],
                    Edition::Java => vec![format!(
                        "title {sel} {loc} {{\"text\":\"{}\"}}",
                        escape_json(&text)
                    )],
                }
            }
            "tellraw" => {
                let sel = args
                    .first()
                    .and_then(|e| self.selector_of(e))
                    .map(|s| s.emit())
                    .unwrap_or_else(|| "@a".into());
                let text = args
                    .get(1)
                    .and_then(|e| match e {
                        Expr::Call { callee, args } => {
                            if let Expr::Field { name, .. } = callee.as_ref() {
                                if name == "raw" {
                                    return string_lit(&args[0]);
                                }
                            }
                            string_lit(e)
                        }
                        _ => string_lit(e),
                    })
                    .unwrap_or_default();
                match self.config.edition {
                    Edition::Bedrock => vec![format!(
                        "tellraw {sel} {{\"rawtext\":[{{\"text\":\"{}\"}}]}}",
                        escape_json(&text)
                    )],
                    Edition::Java => vec![format!(
                        "tellraw {sel} {{\"text\":\"{}\"}}",
                        escape_json(&text)
                    )],
                }
            }
            "random" => {
                if self.config.edition == Edition::Java {
                    self.errors
                        .push(Diagnostic::new(0..0, "`random` is Bedrock-only"));
                    return Vec::new();
                }
                let lo = args
                    .first()
                    .and_then(|e| eval_int(e, &self.enums))
                    .unwrap_or(0);
                let hi = args
                    .get(1)
                    .and_then(|e| eval_int(e, &self.enums))
                    .unwrap_or(0);
                vec![format!(
                    "scoreboard players random {} mt {lo} {hi}",
                    self.this_sel
                )]
            }
            _ => Vec::new(),
        }
    }

    fn lvalue(&self, expr: &Expr) -> Option<(String, FieldMeta)> {
        match expr {
            Expr::Field { base, name } => {
                let owner = match base.as_ref() {
                    Expr::This => self.current_class.clone(),
                    Expr::Ident(n) if self.classes.contains_key(n) => n.clone(),
                    Expr::Ident(n) => {
                        if let Some(ty) = self.bindings.get(n) {
                            let _ = ty;
                        }
                        self.current_class.clone()
                    }
                    other => self.call_owner(other)?,
                };
                let mut owner = owner;
                if let Expr::Ident(n) = base.as_ref() {
                    if self.classes.contains_key(n) {
                        owner = n.clone();
                    }
                }
                let field = self.fields.get(&(owner, name.clone()))?;
                let holder = if field.is_static {
                    self.config.edition.world_holder().to_string()
                } else {
                    match base.as_ref() {
                        Expr::This => self.this_sel.clone(),
                        Expr::Ident(n) => self
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
        match expr {
            Expr::Ident(name) => {
                if let Some(sel) = self.bindings.get(name) {
                    return Some(Selector::simple(sel));
                }
                None
            }
            Expr::This => Some(Selector::simple(&self.this_sel)),
            Expr::Call { callee, args } => {
                if let Expr::Field { base, name } = callee.as_ref() {
                    if let Expr::Ident(recv) = base.as_ref() {
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
                sel.args.push(("tag".into(), string_lit(&args[0])?));
            }
            "withoutTag" => {
                sel.args
                    .push(("tag".into(), format!("!{}", string_lit(&args[0])?)));
            }
            "inBox" => {
                let nums: Vec<i64> = args
                    .iter()
                    .filter_map(|e| eval_int(e, &self.enums))
                    .collect();
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
                let r = eval_int(args.get(1)?, &self.enums)?;
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
                let item = item_id(args.first());
                let n = args
                    .get(1)
                    .and_then(|e| eval_int(e, &self.enums))
                    .unwrap_or(1);
                match self.config.edition {
                    Edition::Bedrock => sel
                        .args
                        .push(("hasitem".into(), format!("{{item={item},quantity={n}..}}"))),
                    Edition::Java => {
                        let item = if item.contains(':') {
                            item
                        } else {
                            format!("minecraft:{item}")
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
        if let Expr::Field { base, name } = expr {
            if let Expr::Ident(c) = base.as_ref() {
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
        if let Expr::Ident(n) = expr {
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
        fn walk(expr: &Expr, x: &mut String, y: &mut String, z: &mut String) -> bool {
            match expr {
                Expr::Call { callee, args } => {
                    if let Expr::Field { base, name } = callee.as_ref() {
                        if let Expr::Ident(recv) = base.as_ref() {
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
            } else if let Expr::Binary {
                op: BinOp::And,
                lhs,
                rhs,
            } = cond
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
        match expr {
            Expr::Binary {
                op: BinOp::And,
                lhs,
                rhs,
            } if !invert => {
                let mut a = self.and_clauses(lhs, false);
                a.extend(self.and_clauses(rhs, false));
                a
            }
            other => self.cond_clauses(other, invert),
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
        match expr {
            Expr::Binary {
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
            Expr::Binary {
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
            Expr::Unary {
                op: UnaryOp::Not,
                expr,
            } => self.cond_clauses(expr, !invert),
            Expr::Binary { op, lhs, rhs } if is_cmp(*op) => {
                if let Some(block) = self.block_eq(lhs, rhs, *op) {
                    let word = if invert { "unless" } else { "if" };
                    return vec![format!("{word} block {block}")];
                }
                if let Some(cl) = self.score_cmp(lhs, rhs, *op, invert) {
                    return vec![cl];
                }
                Vec::new()
            }
            Expr::Field { .. } => {
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
            Expr::Call { callee, args } => {
                if let Expr::Field { base, name } = callee.as_ref() {
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
                    if name == "in" {
                        if let Some(mut sel) = self.selector_of(base) {
                            if self.apply_sel(&mut sel, "in", args).is_some() {
                                let word = if invert { "unless" } else { "if" };
                                return vec![format!("{word} entity {}", sel.emit())];
                            }
                        }
                    }
                }
                if let Expr::Ident(n) = callee.as_ref() {
                    if n == "block" {
                        if let Some(pos) = args.first().and_then(|e| self.pos_of(e)) {
                            let word = if invert { "unless" } else { "if" };
                            return vec![format!("{word} block {pos} air")];
                        }
                    }
                }
                Vec::new()
            }
            Expr::Bool(true) if !invert => Vec::new(),
            Expr::Bool(false) => vec!["if entity @s[tag=__never]".into()],
            _ => Vec::new(),
        }
    }

    fn score_cmp(&self, lhs: &Expr, rhs: &Expr, op: BinOp, invert: bool) -> Option<String> {
        let (holder, field) = self.lvalue(lhs)?;
        if field.kind != SymbolKind::Objective {
            return None;
        }
        let n = eval_int(rhs, &self.enums)?;
        let range = match op {
            BinOp::Eq => format!("{n}"),
            BinOp::Ne => format!("{n}"),
            BinOp::Ge => format!("{n}.."),
            BinOp::Gt => format!("{}..", n + 1),
            BinOp::Le => format!("..{n}"),
            BinOp::Lt => format!("..{}", n - 1),
            BinOp::In => {
                if let Expr::Binary {
                    op: BinOp::Range,
                    lhs,
                    rhs,
                } = rhs
                {
                    let a = eval_int(lhs, &self.enums)?;
                    let b = eval_int(rhs, &self.enums)?;
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
        let Expr::Call { callee, args } = lhs else {
            return None;
        };
        let Expr::Ident(n) = callee.as_ref() else {
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

fn peel_stmt_meta(stmt: &Stmt) -> (Stmt, ChainCmdMeta) {
    let mut meta = ChainCmdMeta::default();
    let mut cur = stmt;
    loop {
        match cur {
            Stmt::Annotated { annotations, inner } => {
                for a in annotations {
                    match a.name.as_str() {
                        "Delay" => {
                            if let Some(Expr::Int(n)) = a.args.first() {
                                meta.delay = *n as u32;
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
            Stmt::Context { prefixes, body } => {
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
            other => return (other.clone(), meta),
        }
    }
}

fn triple_ints(expr: &Expr) -> Option<[i32; 3]> {
    match expr {
        Expr::Call { callee, args } => {
            if let Expr::Field { name, .. } = callee.as_ref() {
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
    match expr {
        Expr::Binary {
            op: BinOp::Or,
            lhs,
            rhs,
        } => {
            let mut v = flatten_or(lhs);
            v.extend(flatten_or(rhs));
            v
        }
        other => vec![other],
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
    matches!(body, [Stmt::Return(None)])
}

fn is_cmp(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::In
    )
}

fn named(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Named(p) => p.parts.last().cloned().unwrap_or_default(),
        TypeRef::Seq(inner) => named(inner),
        TypeRef::Int => "int".into(),
        TypeRef::Boolean => "boolean".into(),
        TypeRef::Void => "void".into(),
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String(s) => Some(s.clone()),
        Expr::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => Some(format!("{}{}", string_lit(lhs)?, string_lit(rhs)?)),
        _ => None,
    }
}

fn eval_int(expr: &Expr, enums: &HashMap<String, Vec<String>>) -> Option<i64> {
    match expr {
        Expr::Int(n) => Some(*n),
        Expr::Bool(true) => Some(1),
        Expr::Bool(false) => Some(0),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
        } => Some(-eval_int(expr, enums)?),
        Expr::Field { base, name } => {
            if let Expr::Ident(en) = base.as_ref() {
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
    match expr {
        Expr::Int(n) => Some(ConstVal::Int(*n)),
        Expr::Bool(b) => Some(ConstVal::Bool(*b)),
        Expr::String(s) => Some(ConstVal::Str(s.clone())),
        Expr::Call { callee, args } => {
            if let Expr::Field { base, name } = callee.as_ref() {
                if let Expr::Ident(recv) = base.as_ref() {
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
                }
            }
            None
        }
        _ => None,
    }
}

fn render_lit(expr: &Expr) -> String {
    match expr {
        Expr::Bool(b) => b.to_string(),
        Expr::Int(n) => n.to_string(),
        Expr::String(s) => s.clone(),
        _ => String::new(),
    }
}

fn enum_or_name(expr: Option<&Expr>) -> String {
    match expr {
        Some(Expr::Field { name, .. }) => name.clone(),
        Some(Expr::Ident(n)) => n.clone(),
        Some(Expr::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn item_id(expr: Option<&Expr>) -> String {
    match expr {
        Some(Expr::Field { name, .. }) => name.to_lowercase(),
        Some(Expr::Ident(n)) => n.to_lowercase(),
        Some(Expr::String(s)) => s.clone(),
        _ => "air".into(),
    }
}

fn block_id(expr: &Expr) -> String {
    match expr {
        Expr::Field { name, .. } => name.to_lowercase(),
        Expr::Ident(n) => n.to_lowercase(),
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

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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
