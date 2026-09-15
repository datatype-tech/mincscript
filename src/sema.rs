//! Semantic checks: private fields/methods, locals, and edition-aware builtins.

use crate::ast::{
    CompilationUnit, Expr, ExprKind, Item, Member, Stmt, StmtKind, TypeRef, Visibility,
};
use crate::config::{Edition, MincConfig};
use crate::diagnostic::Diagnostic;
use crate::span::Span;

#[derive(Clone)]
struct FieldInfo {
    class: String,
    name: String,
    vis: Visibility,
    ty: TypeRef,
    #[allow(dead_code)]
    is_static: bool,
}

#[derive(Clone)]
struct MethodInfo {
    class: String,
    name: String,
    vis: Visibility,
    return_ty: TypeRef,
    #[allow(dead_code)]
    arity: usize,
    #[allow(dead_code)]
    is_static: bool,
    annotations: Vec<String>,
    span: Span,
}

/// Type-check a compilation unit (single file or merged project).
pub fn check(unit: &CompilationUnit) -> Vec<Diagnostic> {
    check_with_config(unit, None)
}

pub fn check_with_config(unit: &CompilationUnit, config: Option<&MincConfig>) -> Vec<Diagnostic> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut enums = Vec::new();
    let mut classes = Vec::new();
    for item in &unit.items {
        match item {
            Item::Class(class) => {
                classes.push(class.name.clone());
                for member in &class.members {
                    match member {
                        Member::Field(f) => fields.push(FieldInfo {
                            class: class.name.clone(),
                            name: f.name.clone(),
                            vis: f.vis,
                            ty: f.ty.clone(),
                            is_static: f.is_static,
                        }),
                        Member::Method(m) => methods.push(MethodInfo {
                            class: class.name.clone(),
                            name: m.name.clone(),
                            vis: m.vis,
                            return_ty: m.return_ty.clone(),
                            arity: m.params.len(),
                            is_static: m.is_static,
                            annotations: m.annotations.iter().map(|a| a.name.clone()).collect(),
                            span: m.span.clone(),
                        }),
                    }
                }
            }
            Item::Enum(en) => enums.push(en.name.clone()),
            _ => {}
        }
    }

    let mut errors = Vec::new();
    for (i, a) in methods.iter().enumerate() {
        for b in methods.iter().skip(i + 1) {
            if a.class == b.class && a.name == b.name && a.arity == b.arity {
                errors.push(Diagnostic::new(
                    a.span.clone(),
                    format!(
                        "duplicate method `{}.{}` with arity {}",
                        a.class, a.name, a.arity
                    ),
                ));
            }
        }
    }
    if let Some(cfg) = config {
        let pack = unit.pack.dotted();
        if pack != cfg.pack && !pack.starts_with(&format!("{}.", cfg.pack)) {
            errors.push(Diagnostic::new(
                unit.pack_span.clone(),
                format!(
                    "pack `{pack}` must equal `{0}` or a subpackage of it",
                    cfg.pack
                ),
            ));
        }
        if unit
            .items
            .iter()
            .filter(|i| matches!(i, Item::World(_)))
            .count()
            > 1
        {
            errors.push(Diagnostic::new(
                unit.pack_span.clone(),
                "project must contain exactly one world controller",
            ));
        }
        let ticking = unit.items.iter().filter_map(|i| match i {
            Item::World(w) => Some(
                w.clauses
                    .iter()
                    .filter(|c| matches!(c, crate::ast::WorldClause::TickingArea { .. }))
                    .count(),
            ),
            _ => None,
        });
        let ticking_n: usize = ticking.sum();
        if cfg.edition == Edition::Bedrock && ticking_n > 10 {
            errors.push(Diagnostic::new(
                unit.pack_span.clone(),
                "Bedrock ticking areas would exceed 10",
            ));
        }
    }

    for item in &unit.items {
        match item {
            Item::Class(class) => {
                for member in &class.members {
                    match member {
                        Member::Method(method) => {
                            let mut locals: Vec<(String, TypeRef)> = method
                                .params
                                .iter()
                                .map(|p| (p.name.clone(), p.ty.clone()))
                                .collect();
                            let java_only = method.annotations.iter().any(|a| a.name == "JavaOnly");
                            let bedrock_only =
                                method.annotations.iter().any(|a| a.name == "BedrockOnly");
                            check_stmts(
                                &class.name,
                                &method.body,
                                &fields,
                                &methods,
                                &classes,
                                &enums,
                                &mut locals,
                                &mut errors,
                                config,
                                java_only,
                                bedrock_only,
                            );
                        }
                        Member::Field(f) => {
                            if let Some(init) = &f.init {
                                let locals = Vec::new();
                                check_expr(
                                    &class.name,
                                    init,
                                    &fields,
                                    &methods,
                                    &classes,
                                    &enums,
                                    &locals,
                                    &mut errors,
                                    config,
                                    false,
                                    false,
                                );
                            }
                        }
                    }
                }
            }
            Item::Chain(chain) => {
                let mut locals = Vec::new();
                check_stmts(
                    "",
                    &chain.body,
                    &fields,
                    &methods,
                    &classes,
                    &enums,
                    &mut locals,
                    &mut errors,
                    config,
                    false,
                    false,
                );
            }
            _ => {}
        }
    }
    if let Some(file) = &unit.file {
        for e in &mut errors {
            if e.file.is_none() {
                e.file = Some(file.clone());
            }
        }
    }
    errors
}

#[allow(clippy::too_many_arguments)]
fn check_stmts(
    current_class: &str,
    stmts: &[Stmt],
    fields: &[FieldInfo],
    methods: &[MethodInfo],
    classes: &[String],
    enums: &[String],
    locals: &mut Vec<(String, TypeRef)>,
    errors: &mut Vec<Diagnostic>,
    config: Option<&MincConfig>,
    java_only: bool,
    bedrock_only: bool,
) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Annotated { inner, .. } => {
                check_stmts(
                    current_class,
                    std::slice::from_ref(inner.as_ref()),
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
            }
            StmtKind::Local { ty, name, init } => {
                if let Some(init) = init {
                    check_expr(
                        current_class,
                        init,
                        fields,
                        methods,
                        classes,
                        enums,
                        locals,
                        errors,
                        config,
                        java_only,
                        bedrock_only,
                    );
                }
                locals.push((name.clone(), ty.clone()));
            }
            StmtKind::Foreach {
                ty,
                name,
                iter,
                body,
            } => {
                check_expr(
                    current_class,
                    iter,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                locals.push((name.clone(), ty.clone()));
                check_stmts(
                    current_class,
                    body,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                locals.pop();
            }
            StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                check_expr(
                    current_class,
                    cond,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                check_stmts(
                    current_class,
                    then_body,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                if let Some(else_body) = else_body {
                    check_stmts(
                        current_class,
                        else_body,
                        fields,
                        methods,
                        classes,
                        enums,
                        locals,
                        errors,
                        config,
                        java_only,
                        bedrock_only,
                    );
                }
            }
            StmtKind::Context { prefixes, body } => {
                for prefix in prefixes {
                    match prefix {
                        crate::ast::ContextPrefix::As(e)
                        | crate::ast::ContextPrefix::At(e)
                        | crate::ast::ContextPrefix::Facing(e) => {
                            check_expr(
                                current_class,
                                e,
                                fields,
                                methods,
                                classes,
                                enums,
                                locals,
                                errors,
                                config,
                                java_only,
                                bedrock_only,
                            );
                        }
                        _ => {}
                    }
                }
                check_stmts(
                    current_class,
                    body,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
            }
            StmtKind::Switch {
                expr,
                arms,
                default,
            } => {
                check_expr(
                    current_class,
                    expr,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                if type_of(current_class, expr, fields, methods, classes, enums, locals).as_deref()
                    == Some("String")
                {
                    errors.push(Diagnostic::new(
                        expr.span.clone(),
                        "`switch` on `String` is not valid (strings are compile-time; switch on enum/int)",
                    ));
                }
                for (_, body) in arms {
                    check_stmts(
                        current_class,
                        body,
                        fields,
                        methods,
                        classes,
                        enums,
                        locals,
                        errors,
                        config,
                        java_only,
                        bedrock_only,
                    );
                }
                if let Some(body) = default {
                    check_stmts(
                        current_class,
                        body,
                        fields,
                        methods,
                        classes,
                        enums,
                        locals,
                        errors,
                        config,
                        java_only,
                        bedrock_only,
                    );
                }
            }
            StmtKind::Return(Some(expr)) | StmtKind::Expr(expr) => {
                check_expr(
                    current_class,
                    expr,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
            }
            StmtKind::Assign { target, value, .. } => {
                check_expr(
                    current_class,
                    target,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                check_expr(
                    current_class,
                    value,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
                if let ExprKind::Field { base, name } = &target.kind {
                    if let Some(owner) =
                        type_of(current_class, base, fields, methods, classes, enums, locals)
                    {
                        deny_private_field(
                            current_class,
                            &owner,
                            name,
                            fields,
                            errors,
                            target.span.clone(),
                        );
                    }
                }
            }
            StmtKind::Return(None) | StmtKind::Label(_) => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_expr(
    current_class: &str,
    expr: &Expr,
    fields: &[FieldInfo],
    methods: &[MethodInfo],
    classes: &[String],
    enums: &[String],
    locals: &[(String, TypeRef)],
    errors: &mut Vec<Diagnostic>,
    config: Option<&MincConfig>,
    java_only: bool,
    bedrock_only: bool,
) {
    match &expr.kind {
        ExprKind::Unary { expr, .. } => check_expr(
            current_class,
            expr,
            fields,
            methods,
            classes,
            enums,
            locals,
            errors,
            config,
            java_only,
            bedrock_only,
        ),
        ExprKind::Binary { op, lhs, rhs, .. } => {
            check_expr(
                current_class,
                lhs,
                fields,
                methods,
                classes,
                enums,
                locals,
                errors,
                config,
                java_only,
                bedrock_only,
            );
            check_expr(
                current_class,
                rhs,
                fields,
                methods,
                classes,
                enums,
                locals,
                errors,
                config,
                java_only,
                bedrock_only,
            );
            if *op == crate::ast::BinOp::Add
                && (matches!(&lhs.kind, ExprKind::String(_))
                    || matches!(&rhs.kind, ExprKind::String(_)))
                && !(is_string_const(lhs) && is_string_const(rhs))
            {
                errors.push(Diagnostic::new(
                    expr.span.clone(),
                    "String concatenation is compile-time only",
                ));
            }
        }
        ExprKind::Call { callee, args } => {
            check_expr(
                current_class,
                callee,
                fields,
                methods,
                classes,
                enums,
                locals,
                errors,
                config,
                java_only,
                bedrock_only,
            );
            for arg in args {
                check_expr(
                    current_class,
                    arg,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
            }
            if let ExprKind::Ident(name) = &callee.kind {
                if name == "random" {
                    if let Some(cfg) = config {
                        if cfg.edition == Edition::Java {
                            errors.push(Diagnostic::new(
                                expr.span.clone(),
                                "`random(min, max)` is Bedrock-only (`scoreboard players random`)",
                            ));
                        }
                    }
                }
                if name == "cmd" && config.map(|c| c.strict_raw).unwrap_or(false) {
                    errors.push(Diagnostic::new(
                        expr.span.clone(),
                        "`cmd` is forbidden under --strict-raw",
                    ));
                }
            }
            if let ExprKind::Field { base, name } = &callee.kind {
                if name == "data" {
                    if let Some(cfg) = config {
                        if cfg.edition == Edition::Java {
                            errors.push(Diagnostic::new(
                                expr.span.clone(),
                                "`.data(...)` is Bedrock aux-value syntax; invalid on Java",
                            ));
                        }
                    }
                }
                let owner = if let ExprKind::Ident(class) = &base.kind {
                    Some(class.clone())
                } else {
                    type_of(current_class, base, fields, methods, classes, enums, locals)
                };
                if let Some(owner) = owner {
                    deny_private_method(
                        current_class,
                        &owner,
                        name,
                        methods,
                        errors,
                        expr.span.clone(),
                    );
                    let overloads: Vec<_> = methods
                        .iter()
                        .filter(|m| m.class == owner && m.name == *name)
                        .collect();
                    if !overloads.is_empty() && !overloads.iter().any(|m| m.arity == args.len()) {
                        errors.push(Diagnostic::new(
                            expr.span.clone(),
                            format!("no overload of `{owner}.{name}` with arity {}", args.len()),
                        ));
                    }
                    if let Some(m) = overloads.iter().find(|m| m.arity == args.len()) {
                        if m.annotations.iter().any(|a| a == "JavaOnly") {
                            if !java_only {
                                errors.push(Diagnostic::new(
                                    expr.span.clone(),
                                    format!(
                                        "`{}.{name}` is @JavaOnly and must not be called from shared code",
                                        m.class
                                    ),
                                ));
                            }
                            if let Some(cfg) = config {
                                if cfg.edition == Edition::Bedrock {
                                    errors.push(Diagnostic::new(
                                        expr.span.clone(),
                                        format!(
                                            "`{}.{name}` is @JavaOnly (target edition is bedrock)",
                                            m.class
                                        ),
                                    ));
                                }
                            }
                        }
                        if m.annotations.iter().any(|a| a == "BedrockOnly") {
                            if !bedrock_only {
                                errors.push(Diagnostic::new(
                                    expr.span.clone(),
                                    format!(
                                        "`{}.{name}` is @BedrockOnly and must not be called from shared code",
                                        m.class
                                    ),
                                ));
                            }
                            if let Some(cfg) = config {
                                if cfg.edition == Edition::Java {
                                    errors.push(Diagnostic::new(
                                        expr.span.clone(),
                                        format!(
                                            "`{}.{name}` is @BedrockOnly (target edition is java)",
                                            m.class
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        ExprKind::Field { base, name } => {
            check_expr(
                current_class,
                base,
                fields,
                methods,
                classes,
                enums,
                locals,
                errors,
                config,
                java_only,
                bedrock_only,
            );
            if let Some(owner) =
                type_of(current_class, base, fields, methods, classes, enums, locals)
            {
                deny_private_field(
                    current_class,
                    &owner,
                    name,
                    fields,
                    errors,
                    expr.span.clone(),
                );
            }
        }
        ExprKind::Null => {
            errors.push(Diagnostic::new(
                expr.span.clone(),
                "`null` is not valid; missing selectors use `exists() == false`",
            ));
        }
        ExprKind::New { ty, args } => {
            for arg in args {
                check_expr(
                    current_class,
                    arg,
                    fields,
                    methods,
                    classes,
                    enums,
                    locals,
                    errors,
                    config,
                    java_only,
                    bedrock_only,
                );
            }
            let name = named_type(ty).unwrap_or_default();
            if matches!(name.as_str(), "Player" | "Entity") || classes.iter().any(|c| c == &name) {
                errors.push(Diagnostic::new(
                    expr.span.clone(),
                    format!("`new {name}()` is not valid; players are not allocated"),
                ));
            } else if !matches!(
                name.as_str(),
                "Region" | "BlockPos" | "Item" | "Block" | "Layout" | "ItemStack"
            ) && !name.is_empty()
            {
                errors.push(Diagnostic::new(
                    expr.span.clone(),
                    format!("`new` is only for compile-time descriptors, not `{name}`"),
                ));
            }
        }
        _ => {}
    }
}

fn type_of(
    current_class: &str,
    expr: &Expr,
    fields: &[FieldInfo],
    methods: &[MethodInfo],
    classes: &[String],
    enums: &[String],
    locals: &[(String, TypeRef)],
) -> Option<String> {
    match &expr.kind {
        ExprKind::This => {
            if current_class.is_empty() {
                None
            } else {
                Some(current_class.to_string())
            }
        }
        ExprKind::String(_) => Some("String".into()),
        ExprKind::Int(_) => Some("int".into()),
        ExprKind::Bool(_) => Some("boolean".into()),
        ExprKind::Ident(name) => {
            if let Some((_, ty)) = locals.iter().find(|(n, _)| n == name) {
                return named_type(ty);
            }
            if classes.iter().any(|c| c == name) || fields.iter().any(|f| f.class == *name) {
                return Some(name.clone());
            }
            if enums.iter().any(|e| e == name) {
                return Some(name.clone());
            }
            None
        }
        ExprKind::Field { base, name } => {
            if let ExprKind::Ident(class) = &base.kind {
                if classes.iter().any(|c| c == class) || fields.iter().any(|f| f.class == *class) {
                    if let Some(f) = fields.iter().find(|f| f.class == *class && f.name == *name) {
                        return named_type(&f.ty);
                    }
                    return Some(class.clone());
                }
                if enums.iter().any(|e| e == class) {
                    return Some("int".into());
                }
            }
            if let Some(owner) =
                type_of(current_class, base, fields, methods, classes, enums, locals)
            {
                if let Some(f) = fields.iter().find(|f| f.class == owner && f.name == *name) {
                    return named_type(&f.ty);
                }
                return Some(owner);
            }
            None
        }
        ExprKind::Call { callee, .. } => {
            if let ExprKind::Field { base, name } = &callee.kind {
                if name == "of" {
                    if let ExprKind::Ident(class) = &base.kind {
                        if classes.iter().any(|c| c == class) {
                            return Some(class.clone());
                        }
                    }
                }
                let owner = match &base.kind {
                    ExprKind::Ident(class)
                        if methods.iter().any(|m| m.class == *class)
                            || classes.iter().any(|c| c == class)
                            || matches!(
                                class.as_str(),
                                "Player"
                                    | "Players"
                                    | "BlockPos"
                                    | "Region"
                                    | "Items"
                                    | "Blocks"
                                    | "World"
                                    | "Text"
                            ) =>
                    {
                        Some(class.clone())
                    }
                    _ => type_of(current_class, base, fields, methods, classes, enums, locals),
                };
                if let Some(owner) = owner {
                    if let Some(m) = methods.iter().find(|m| m.class == owner && m.name == *name) {
                        return named_type(&m.return_ty);
                    }
                    return builtin_return(&owner, name);
                }
            }
            if let ExprKind::Ident(name) = &callee.kind {
                return match name.as_str() {
                    "block" => Some("Block".into()),
                    "title" | "tellraw" => Some("void".into()),
                    "random" => Some("int".into()),
                    _ => None,
                };
            }
            None
        }
        ExprKind::New { ty, .. } => named_type(ty),
        _ => None,
    }
}

fn builtin_return(owner: &str, name: &str) -> Option<String> {
    Some(
        match (owner, name) {
            ("Player", "self") | ("Player", "nearest") => "Player",
            ("Players", "all") => "Player",
            ("Players", "entities") => "Entity",
            ("BlockPos", "of")
            | ("BlockPos", "here")
            | ("BlockPos", "up")
            | ("BlockPos", "down") => "BlockPos",
            ("Region", "box") | ("Region", "circle") => "Region",
            ("Items", _) | ("Blocks", _) => owner,
            _ => return None,
        }
        .to_string(),
    )
}

fn is_string_const(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::String(_) => true,
        ExprKind::Binary {
            op: crate::ast::BinOp::Add,
            lhs,
            rhs,
        } => is_string_const(lhs) && is_string_const(rhs),
        _ => false,
    }
}

fn named_type(ty: &TypeRef) -> Option<String> {
    match ty {
        TypeRef::Named(path) => Some(path.parts.last().cloned().unwrap_or_default()),
        TypeRef::Seq(inner) => named_type(inner),
        TypeRef::Int => Some("int".into()),
        TypeRef::Boolean => Some("boolean".into()),
        TypeRef::Void => Some("void".into()),
    }
}

fn deny_private_field(
    current_class: &str,
    owner: &str,
    field_name: &str,
    fields: &[FieldInfo],
    errors: &mut Vec<Diagnostic>,
    span: Span,
) {
    for f in fields {
        if f.class == owner
            && f.name == field_name
            && f.vis == Visibility::Private
            && f.class != current_class
        {
            errors.push(Diagnostic::new(
                span.clone(),
                format!(
                    "private field `{}.{field_name}` is not accessible here",
                    f.class
                ),
            ));
        }
    }
}

fn deny_private_method(
    current_class: &str,
    owner: &str,
    method_name: &str,
    methods: &[MethodInfo],
    errors: &mut Vec<Diagnostic>,
    span: Span,
) {
    for m in methods {
        if m.class == owner
            && m.name == method_name
            && m.vis == Visibility::Private
            && m.class != current_class
        {
            errors.push(Diagnostic::new(
                span.clone(),
                format!(
                    "private method `{}.{method_name}` is not accessible here",
                    m.class
                ),
            ));
        }
    }
}
