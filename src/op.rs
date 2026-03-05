use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;

pub fn optimize_typed_ast(node: &TypedExpression) -> TypedExpression {
    let optimized_children = node.children
        .iter()
        .map(optimize_typed_ast)
        .collect::<Vec<_>>();
    let rebuilt_expr = rebuild_expr_from_children(&node.expr, &optimized_children);
    let rebuilt_node = TypedExpression {
        expr: rebuilt_expr,
        typ: node.typ.clone(),
        children: optimized_children,
    };
    fold_constants(rebuilt_node)
}

fn rebuild_expr_from_children(expr: &Expression, children: &[TypedExpression]) -> Expression {
    match expr {
        Expression::Apply(items) if items.len() == children.len() =>
            Expression::Apply(children.iter().map(|ch| ch.expr.clone()).collect()),
        _ => expr.clone(),
    }
}

fn fold_constants(node: TypedExpression) -> TypedExpression {
    let items = match &node.expr {
        Expression::Apply(items) => items.clone(),
        _ => return node,
    };
    let Some(Expression::Word(op)) = items.first() else {
        return node;
    };

    match op.as_str() {
        "do" => fold_do(node, &items),
        "if" => fold_if(node, &items),
        "and" => fold_and(node, &items),
        "or" => fold_or(node, &items),
        "not" => fold_not(node, &items),

        "+" | "+#" => fold_int_add(node, &items),
        "-" | "-#" => fold_int_bin(node, &items, i32::wrapping_sub),
        "*" | "*#" => fold_int_mul(node, &items),
        "/" | "/#" => fold_int_checked_bin(node, &items, i32::checked_div),
        "mod" => fold_int_checked_bin(node, &items, i32::checked_rem),

        "=" | "=?" | "=#" => fold_int_cmp(node, &items, |a, b| a == b),
        "<" | "<#" => fold_int_cmp(node, &items, |a, b| a < b),
        ">" | ">#" => fold_int_cmp(node, &items, |a, b| a > b),
        "<=" | "<=#" => fold_int_cmp(node, &items, |a, b| a <= b),
        ">=" | ">=#" => fold_int_cmp(node, &items, |a, b| a >= b),

        "+." => fold_float_bin(node, &items, |a, b| a + b),
        "-." => fold_float_bin(node, &items, |a, b| a - b),
        "*." => fold_float_bin(node, &items, |a, b| a * b),
        "/." => fold_float_bin(node, &items, |a, b| a / b),
        "mod." => fold_float_bin(node, &items, |a, b| a - (a / b).trunc() * b),

        "=." => fold_float_cmp(node, &items, |a, b| a == b),
        "<." => fold_float_cmp(node, &items, |a, b| a < b),
        ">." => fold_float_cmp(node, &items, |a, b| a > b),
        "<=." => fold_float_cmp(node, &items, |a, b| a <= b),
        ">=." => fold_float_cmp(node, &items, |a, b| a >= b),

        "Int->Float" => fold_int_to_float(node, &items),
        "Float->Int" => fold_float_to_int(node, &items),

        _ => node,
    }
}

fn fold_do(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() <= 1 {
        return node;
    }
    if node.children.len() != items.len() {
        return node;
    }

    let last_idx = items.len() - 1;
    let mut kept_indices: Vec<usize> = Vec::new();
    kept_indices.push(0); // keep "do"
    for i in 1..last_idx {
        if !is_pure_literal_expr(&items[i]) {
            kept_indices.push(i);
        }
    }
    kept_indices.push(last_idx);

    if kept_indices.len() == items.len() {
        return node;
    }

    // (do x) => x
    if kept_indices.len() == 2 {
        let only_expr_idx = kept_indices[1];
        return node.children.get(only_expr_idx).cloned().unwrap_or(node);
    }

    let new_expr_items = kept_indices
        .iter()
        .filter_map(|idx| items.get(*idx).cloned())
        .collect::<Vec<_>>();
    let new_children = kept_indices
        .iter()
        .filter_map(|idx| node.children.get(*idx).cloned())
        .collect::<Vec<_>>();

    TypedExpression {
        expr: Expression::Apply(new_expr_items),
        typ: node.typ,
        children: new_children,
    }
}

fn fold_int_add(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(0) = items.get(1).and_then(int_literal) {
        return node.children.get(2).cloned().unwrap_or(node);
    }
    if let Some(0) = items.get(2).and_then(int_literal) {
        return node.children.get(1).cloned().unwrap_or(node);
    }
    fold_int_bin(node, items, i32::wrapping_add)
}

fn fold_int_mul(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(1) = items.get(1).and_then(int_literal) {
        return node.children.get(2).cloned().unwrap_or(node);
    }
    if let Some(1) = items.get(2).and_then(int_literal) {
        return node.children.get(1).cloned().unwrap_or(node);
    }
    fold_int_bin(node, items, i32::wrapping_mul)
}

fn fold_if(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 4 {
        return node;
    }
    let Some(cond) = items.get(1).and_then(bool_literal) else {
        return node;
    };
    if cond {
        node.children.get(2).cloned().unwrap_or(node)
    } else {
        node.children.get(3).cloned().unwrap_or(node)
    }
}

fn fold_and(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(lhs) = items.get(1).and_then(bool_literal) {
        if lhs {
            return node.children.get(2).cloned().unwrap_or(node);
        }
        return make_folded_literal(&node, Expression::Word("false".to_string()), Type::Bool);
    }
    node
}

fn fold_or(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(lhs) = items.get(1).and_then(bool_literal) {
        if lhs {
            return make_folded_literal(&node, Expression::Word("true".to_string()), Type::Bool);
        }
        return node.children.get(2).cloned().unwrap_or(node);
    }
    node
}

fn fold_not(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 2 {
        return node;
    }
    let Some(v) = items.get(1).and_then(bool_literal) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word(if !v { "true" } else { "false" }.to_string()),
        Type::Bool
    )
}

fn fold_int_bin(node: TypedExpression, items: &[Expression], f: fn(i32, i32) -> i32) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(int_literal), items.get(2).and_then(int_literal)) else {
        return node;
    };
    make_folded_literal(&node, Expression::Int(f(a, b)), Type::Int)
}

fn fold_int_checked_bin(
    node: TypedExpression,
    items: &[Expression],
    f: fn(i32, i32) -> Option<i32>
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(int_literal), items.get(2).and_then(int_literal)) else {
        return node;
    };
    let Some(v) = f(a, b) else {
        // Preserve runtime semantics for trap cases (divide/rem by zero, overflow).
        return node;
    };
    make_folded_literal(&node, Expression::Int(v), Type::Int)
}

fn fold_int_cmp(
    node: TypedExpression,
    items: &[Expression],
    f: fn(i32, i32) -> bool
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(int_literal), items.get(2).and_then(int_literal)) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word(if f(a, b) { "true" } else { "false" }.to_string()),
        Type::Bool
    )
}

fn fold_float_bin(
    node: TypedExpression,
    items: &[Expression],
    f: fn(f32, f32) -> f32
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(float_literal), items.get(2).and_then(float_literal)) else {
        return node;
    };
    make_folded_literal(&node, Expression::Float(f(a, b)), Type::Float)
}

fn fold_float_cmp(
    node: TypedExpression,
    items: &[Expression],
    f: fn(f32, f32) -> bool
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(float_literal), items.get(2).and_then(float_literal)) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word(if f(a, b) { "true" } else { "false" }.to_string()),
        Type::Bool
    )
}

fn fold_int_to_float(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    let Some(a) = items.get(1).and_then(int_literal) else {
        return node;
    };
    make_folded_literal(&node, Expression::Float(a as f32), Type::Float)
}

fn fold_float_to_int(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    let Some(a) = items.get(1).and_then(float_literal) else {
        return node;
    };
    make_folded_literal(&node, Expression::Int(a as i32), Type::Int)
}

fn make_folded_literal(node: &TypedExpression, expr: Expression, typ: Type) -> TypedExpression {
    TypedExpression {
        expr,
        typ: node.typ.clone().or(Some(typ)),
        children: Vec::new(),
    }
}

fn int_literal(expr: &Expression) -> Option<i32> {
    match expr {
        Expression::Int(v) => Some(*v),
        _ => None,
    }
}

fn float_literal(expr: &Expression) -> Option<f32> {
    match expr {
        Expression::Float(v) => Some(*v),
        _ => None,
    }
}

fn bool_literal(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::Word(w) if w == "true" => Some(true),
        Expression::Word(w) if w == "false" => Some(false),
        _ => None,
    }
}

fn is_pure_literal_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Int(_) | Expression::Float(_) => true,
        Expression::Word(w) if w == "true" || w == "false" => true,
        _ => false,
    }
}
