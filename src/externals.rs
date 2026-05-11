use crate::parser::Expression;
use crate::types::Type;

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
const BUILTIN_HOST_EXTERNS_SRC: &str = r#"
(extern host list_dir list-dir! ([Char] -> [Char]))
(extern host read_file read! ([Char] -> [Char]))
(extern host write_file write! ([Char] [Char] -> ()))
(extern host mkdir_p mkdir! ([Char] -> ()))
(extern host delete delete! ([Char] -> ()))
(extern host move move! ([Char] [Char] -> ()))
(extern host print print! ([Char] -> ()))
(extern host sleep sleep! (Int -> ()))
(extern host clear clear! (() -> ()))
"#;

#[cfg(feature = "io")]
pub fn builtin_host_extern_definitions() -> Result<Vec<Expression>, String> {
    let ast = crate::parser::build_library(BUILTIN_HOST_EXTERNS_SRC)?;
    let Expression::Apply(items) = ast else {
        return Err("builtin host externs did not parse as top-level do expression".to_string());
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return Err("builtin host externs did not parse as top-level do expression".to_string());
    }
    Ok(items.into_iter().skip(1).collect())
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
