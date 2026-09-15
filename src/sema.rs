//! Semantic checks: private fields/methods, locals, and edition-aware builtins.

use crate::ast::{CompilationUnit, Expr, Item, Member, Stmt, TypeRef, Visibility};
use crate::config::{Edition, MincConfig};
use crate::diagnostic::Diagnostic;

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
                        }),
                    }
                }
            }
            Item::Enum(en) => enums.push(en.name.clone()),
            _ => {}
        }
    }

    let mut errors = Vec::new();
    if let Some(cfg) = config {
        let pack = unit.pack.dotted();
        if pack != cfg.pack && !pack.starts_with(&format!("{}.", cfg.pack)) {
            errors.push(Diagnostic::new(
                0..0,
                format!(
                    "pack `{pack}` must equal `{0}` or a subpackage of it",
                    cfg.pack
                ),
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
                );
            }
            _ => {}
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
) {
    for stmt in stmts {
        match stmt {
            Stmt::Annotated { inner, .. } => {
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
                );
            }
            Stmt::Local { ty, name, init } => {
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
                    );
                }
                locals.push((name.clone(), ty.clone()));
            }
            Stmt::Foreach {
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
                );
                locals.pop();
            }
            Stmt::If {
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
                    );
                }
            }
            Stmt::Context { prefixes, body } => {
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
                );
            }
            Stmt::Switch {
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
                );
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
                    );
                }
            }
            Stmt::Return(Some(expr)) | Stmt::Expr(expr) => {
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
                );
            }
            Stmt::Assign { target, value, .. } => {
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
                );
                if let Expr::Field { base, name } = target {
                    if let Some(owner) =
                        type_of(current_class, base, fields, methods, classes, enums, locals)
                    {
                        deny_private_field(current_class, &owner, name, fields, errors);
                    }
                }
            }
            Stmt::Return(None) | Stmt::Label(_) => {}
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
) {
    match expr {
        Expr::Unary { expr, .. } => check_expr(
            current_class,
            expr,
            fields,
            methods,
            classes,
            enums,
            locals,
            errors,
            config,
        ),
        Expr::Binary { lhs, rhs, .. } => {
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
            );
        }
        Expr::Call { callee, args } => {
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
                );
            }
            if let Expr::Ident(name) = callee.as_ref() {
                if name == "random" {
                    if let Some(cfg) = config {
                        if cfg.edition == Edition::Java {
                            errors.push(Diagnostic::new(
                                0..0,
                                "`random(min, max)` is Bedrock-only (`scoreboard players random`)",
                            ));
                        }
                    }
                }
            }
            if let Expr::Field { base, name } = callee.as_ref() {
                if let Some(owner) =
                    type_of(current_class, base, fields, methods, classes, enums, locals)
                {
                    deny_private_method(current_class, &owner, name, methods, errors);
                }
                if let Expr::Ident(class) = base.as_ref() {
                    deny_private_method(current_class, class, name, methods, errors);
                }
            }
        }
        Expr::Field { base, name } => {
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
            );
            if let Some(owner) =
                type_of(current_class, base, fields, methods, classes, enums, locals)
            {
                deny_private_field(current_class, &owner, name, fields, errors);
            }
        }
        Expr::New { args, .. } => {
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
                );
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
    match expr {
        Expr::This => {
            if current_class.is_empty() {
                None
            } else {
                Some(current_class.to_string())
            }
        }
        Expr::Ident(name) => {
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
        Expr::Field { base, name } => {
            if let Expr::Ident(class) = base.as_ref() {
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
        Expr::Call { callee, .. } => {
            if let Expr::Field { base, name } = callee.as_ref() {
                if name == "of" {
                    if let Expr::Ident(class) = base.as_ref() {
                        if classes.iter().any(|c| c == class) {
                            return Some(class.clone());
                        }
                    }
                }
                let owner = match base.as_ref() {
                    Expr::Ident(class)
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
            if let Expr::Ident(name) = callee.as_ref() {
                return match name.as_str() {
                    "block" => Some("Block".into()),
                    "title" | "tellraw" => Some("void".into()),
                    "random" => Some("int".into()),
                    _ => None,
                };
            }
            None
        }
        Expr::New { ty, .. } => named_type(ty),
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
) {
    for f in fields {
        if f.class == owner
            && f.name == field_name
            && f.vis == Visibility::Private
            && f.class != current_class
        {
            errors.push(Diagnostic::new(
                0..0,
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
) {
    for m in methods {
        if m.class == owner
            && m.name == method_name
            && m.vis == Visibility::Private
            && m.class != current_class
        {
            errors.push(Diagnostic::new(
                0..0,
                format!(
                    "private method `{}.{method_name}` is not accessible here",
                    m.class
                ),
            ));
        }
    }
}
