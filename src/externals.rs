use crate::parser::Expression;
use crate::types::Type;

#[derive(Debug, Clone, Copy)]
pub struct BuiltinHostExternSpec {
    pub module: &'static str,
    pub import: &'static str,
    pub local_name: &'static str,
    pub typ: fn() -> Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternDecl {
    pub module: String,
    pub import: String,
    pub local_name: String,
    pub typ: Type,
}

fn parse_type_expr(expr: &Expression) -> Result<Type, String> {
    match expr {
        Expression::Word(name) => match name.as_str() {
            "Int" => Ok(Type::Int),
            "Dec" => Ok(Type::Dec),
            "Bool" => Ok(Type::Bool),
            "Char" => Ok(Type::Char),
            other => Err(format!("Unknown extern type '{}'", other)),
        },
        Expression::Apply(items) if items.is_empty() => Ok(Type::Unit),
        Expression::Apply(items) => {
            if let Some(arrow_idx) = items
                .iter()
                .position(|item| matches!(item, Expression::Word(w) if w == "->"))
            {
                if items.iter().skip(arrow_idx + 1).any(
                    |item| matches!(item, Expression::Word(w) if w == "->")
                ) {
                    return Err(format!(
                        "Extern function type must contain exactly one '->': {}",
                        expr.to_lisp()
                    ));
                }
                if arrow_idx == 0 || arrow_idx + 2 != items.len() {
                    return Err(format!(
                        "Invalid extern function type syntax: {}",
                        expr.to_lisp()
                    ));
                }
                let ret = parse_type_expr(&items[arrow_idx + 1])?;
                let mut out = ret;
                for param_expr in items[..arrow_idx].iter().rev() {
                    let param = parse_type_expr(param_expr)?;
                    out = Type::Function(Box::new(param), Box::new(out));
                }
                return Ok(out);
            }

            if let Some(Expression::Word(head)) = items.first() {
                if matches!(
                    head.as_str(),
                    "vector" | "string" | "integers" | "bools" | "decimals" | "strings"
                ) {
                    if items.len() != 2 {
                        return Err(format!("Invalid list extern type syntax: {}", expr.to_lisp()));
                    }
                    let inner = parse_type_expr(&items[1])?;
                    return Ok(Type::List(Box::new(inner)));
                }
                if head == "tuple" {
                    if items.len() < 2 {
                        return Err(format!("Tuple type must have at least one element: {}", expr.to_lisp()));
                    }
                    let mut elems = Vec::new();
                    for item in &items[1..] {
                        elems.push(parse_type_expr(item)?);
                    }
                    return Ok(Type::Tuple(elems));
                }
            }

            Err(format!("Invalid extern type syntax: {}", expr.to_lisp()))
        }
        _ => Err(format!("Invalid extern type: {}", expr.to_lisp())),
    }
}

fn type_to_expr(typ: &Type) -> Expression {
    match typ {
        Type::Int => Expression::Word("Int".to_string()),
        Type::Dec => Expression::Word("Dec".to_string()),
        Type::Bool => Expression::Word("Bool".to_string()),
        Type::Char => Expression::Word("Char".to_string()),
        Type::Unit => Expression::Apply(Vec::new()),
        Type::List(inner) => Expression::Apply(vec![
            Expression::Word("vector".to_string()),
            type_to_expr(inner),
        ]),
        Type::Tuple(items) => {
            let mut out = vec![Expression::Word("tuple".to_string())];
            out.extend(items.iter().map(type_to_expr));
            Expression::Apply(out)
        }
        Type::Function(_, _) => {
            let mut parts = Vec::new();
            let mut current = typ;
            while let Type::Function(a, b) = current {
                parts.push(type_to_expr(a));
                current = b;
            }
            parts.push(Expression::Word("->".to_string()));
            parts.push(type_to_expr(current));
            Expression::Apply(parts)
        }
        Type::Var(v) => Expression::Word(format!("T{}", v.id)),
    }
}

fn ty_char_list() -> Type {
    Type::List(Box::new(Type::Char))
}

fn ty_unit() -> Type {
    Type::Unit
}

fn ty_int() -> Type {
    Type::Int
}

fn fn1(a: Type, r: Type) -> Type {
    Type::Function(Box::new(a), Box::new(r))
}

fn fn2(a: Type, b: Type, r: Type) -> Type {
    Type::Function(Box::new(a), Box::new(fn1(b, r)))
}

pub fn parse_extern_decl(expr: &Expression) -> Result<Option<ExternDecl>, String> {
    let Expression::Apply(items) = expr else {
        return Ok(None);
    };
    let [Expression::Word(kw), Expression::Word(module), Expression::Word(import), Expression::Word(local_name), typ_expr] =
        &items[..]
    else {
        return Ok(None);
    };
    if kw != "extern" {
        return Ok(None);
    }
    let typ = parse_type_expr(typ_expr)?;
    Ok(Some(ExternDecl {
        module: module.clone(),
        import: import.clone(),
        local_name: local_name.clone(),
        typ,
    }))
}

#[cfg(feature = "io")]
pub const BUILTIN_HOST_EXTERNS: &[BuiltinHostExternSpec] = &[
    BuiltinHostExternSpec {
        module: "host",
        import: "list_dir",
        local_name: "list-dir!",
        typ: || fn1(ty_char_list(), ty_char_list()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "read_file",
        local_name: "read!",
        typ: || fn1(ty_char_list(), ty_char_list()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "write_file",
        local_name: "write!",
        typ: || fn2(ty_char_list(), ty_char_list(), ty_unit()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "mkdir_p",
        local_name: "mkdir!",
        typ: || fn1(ty_char_list(), ty_unit()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "delete",
        local_name: "delete!",
        typ: || fn1(ty_char_list(), ty_unit()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "move",
        local_name: "move!",
        typ: || fn2(ty_char_list(), ty_char_list(), ty_unit()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "print",
        local_name: "print!",
        typ: || fn1(ty_char_list(), ty_unit()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "sleep",
        local_name: "sleep!",
        typ: || fn1(ty_int(), ty_unit()),
    },
    BuiltinHostExternSpec {
        module: "host",
        import: "clear",
        local_name: "clear!",
        typ: || fn1(ty_unit(), ty_unit()),
    },
];

#[cfg(not(feature = "io"))]
pub const BUILTIN_HOST_EXTERNS: &[BuiltinHostExternSpec] = &[];

pub fn is_builtin_host_extern_symbol(name: &str) -> bool {
    BUILTIN_HOST_EXTERNS.iter().any(|spec| spec.local_name == name)
}

#[cfg(feature = "io")]
pub fn builtin_host_extern_definitions() -> Result<Vec<Expression>, String> {
    let mut out = Vec::new();
    for spec in BUILTIN_HOST_EXTERNS {
        out.push(Expression::Apply(vec![
            Expression::Word("extern".to_string()),
            Expression::Word(spec.module.to_string()),
            Expression::Word(spec.import.to_string()),
            Expression::Word(spec.local_name.to_string()),
            type_to_expr(&(spec.typ)()),
        ]));
    }
    Ok(out)
}

#[cfg(not(feature = "io"))]
pub fn builtin_host_extern_definitions() -> Result<Vec<Expression>, String> {
    Ok(Vec::new())
}

pub fn prepend_builtin_host_externs(expr: &Expression) -> Result<Expression, String> {
    let mut items = vec![Expression::Word("do".to_string())];
    items.extend(builtin_host_extern_definitions()?);
    match expr {
        Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "do") => {
            items.extend(xs.iter().skip(1).cloned());
        }
        other => items.push(other.clone()),
    }
    Ok(Expression::Apply(items))
}

pub fn extend_with_builtin_host_externs(defs: &mut Vec<Expression>) -> Result<(), String> {
    let mut prefix = builtin_host_extern_definitions()?;
    if prefix.is_empty() {
        return Ok(());
    }
    prefix.extend(std::mem::take(defs));
    *defs = prefix;
    Ok(())
}
