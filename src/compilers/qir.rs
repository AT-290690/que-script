use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;
use std::collections::HashMap;

fn node_key(node: &TypedExpression) -> usize {
    node as *const TypedExpression as usize
}

fn assign_ids(node: &TypedExpression, ids: &mut HashMap<usize, usize>, next_id: &mut usize) {
    let key = node_key(node);
    if ids.contains_key(&key) {
        return;
    }
    ids.insert(key, *next_id);
    *next_id += 1;
    for child in &node.children {
        assign_ids(child, ids, next_id);
    }
}

fn fmt_node_ref(node: &TypedExpression, ids: &HashMap<usize, usize>) -> String {
    format!("n{}", ids.get(&node_key(node)).copied().unwrap_or(0))
}

fn fmt_expr(node: &TypedExpression, ids: &HashMap<usize, usize>) -> String {
    match &node.expr {
        Expression::Int(n) => format!("const.int {}", n),
        Expression::Float(n) => format!("const.float {:?}", n),
        Expression::Word(w) => format!("sym {}", w),
        Expression::Apply(items) => {
            if items.is_empty() {
                return "apply head=- args=[]".to_string();
            }
            let head = node
                .children
                .first()
                .map(|c| fmt_node_ref(c, ids))
                .unwrap_or_else(|| "-".to_string());
            let args = if node.children.len() <= 1 {
                String::new()
            } else {
                node.children[1..]
                    .iter()
                    .map(|c| fmt_node_ref(c, ids))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let head_word = match &items[0] {
                Expression::Word(w) => w.as_str(),
                _ => "?",
            };
            format!("apply head={} args=[{}] op={}", head, args, head_word)
        }
    }
}

fn render_qir_lines(node: &TypedExpression, ids: &HashMap<usize, usize>, out: &mut Vec<String>) {
    let key = node_key(node);
    let id = *ids.get(&key).unwrap_or(&0);
    let typ = node
        .typ
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "_".to_string());
    let expr = fmt_expr(node, ids);
    out.push(format!("n{} :: {} = {}", id, typ, expr));
    for child in &node.children {
        render_qir_lines(child, ids, out);
    }
}

pub fn compile_program_to_qir_typed(program_type: &Type, typed_ast: &TypedExpression) -> String {
    let mut ids: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0usize;
    assign_ids(typed_ast, &mut ids, &mut next_id);
    let root_id = ids.get(&node_key(typed_ast)).copied().unwrap_or(0);

    let mut lines = Vec::new();
    lines.push("; QIR v1".to_string());
    lines.push("; post-desugar, typed, pre-backend".to_string());
    lines.push(format!("program_type: {}", program_type));
    lines.push(format!("root: n{}", root_id));
    lines.push("nodes:".to_string());
    render_qir_lines(typed_ast, &ids, &mut lines);
    lines.join("\n")
}
