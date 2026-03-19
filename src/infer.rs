use crate::parser::Expression;
use crate::types::{ generalize, Type, TypeEnv, TypeScheme, TypeVar };
use std::collections::{ HashMap, HashSet, VecDeque };
use std::ops::{ BitOr, BitOrAssign };

#[derive(Clone, Debug)]
pub enum TypeErrorVariant {
    Vector,
    Call,
    Source,
    IfBody,
    IfCond,
}

#[derive(Clone, Debug)]
pub struct TypeError {
    pub variant: TypeErrorVariant,
    pub expr: Vec<crate::parser::Expression>,
    pub scope: Option<InferErrorScope>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferErrorScope {
    pub user_top_form: usize,
    pub lambda_path: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct InferErrorInfo {
    pub message: String,
    pub scope: Option<InferErrorScope>,
    pub partial_typed_ast: Option<TypedExpression>,
}

#[derive(Clone, Debug)]
pub struct TypedExpression {
    pub expr: Expression,
    pub typ: Option<Type>,
    pub effect: EffectFlags,
    pub children: Vec<TypedExpression>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct EffectFlags(pub u8);

impl EffectFlags {
    pub const PURE: Self = Self(0);
    pub const MUTATE: Self = Self(1 << 0);
    pub const IO: Self = Self(1 << 1);
    pub const UNKNOWN_CALL: Self = Self(1 << 2);

    pub fn is_pure(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl BitOr for EffectFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for EffectFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

fn expression_id(expr: &Expression) -> usize {
    expr as *const Expression as usize
}

fn build_typed_expression(
    expr: &Expression,
    solved_expr_types: &HashMap<usize, Type>
) -> TypedExpression {
    let children = match expr {
        Expression::Apply(items) =>
            items
                .iter()
                .map(|item| build_typed_expression(item, solved_expr_types))
                .collect(),
        _ => Vec::new(),
    };

    TypedExpression {
        expr: expr.clone(),
        typ: solved_expr_types.get(&expression_id(expr)).cloned(),
        effect: EffectFlags::PURE,
        children,
    }
}

fn is_io_op(op: &str) -> bool {
    matches!(
        op,
        "read!" |
            "write!" |
            "list-dir!" |
            "mkdir!" |
            "delete!" |
            "move!" |
            "print!" |
            "sleep!" |
            "clear!"
    )
}

fn is_mutating_op(op: &str) -> bool {
    matches!(op, "set!" | "alter!" | "pop!")
}

fn is_bang_contract_op(op: &str, known_requires_bang: &HashMap<String, bool>) -> bool {
    if is_mutating_op(op) || op == "push!" {
        return true;
    }
    if known_requires_bang.get(op).copied().unwrap_or(false) {
        return true;
    }
    if op.ends_with('!') {
        return true;
    }
    if is_impure_bang_exception_name(op) {
        return true;
    }
    false
}

fn is_intrinsic_pure_op(op: &str) -> bool {
    matches!(
        op,
        "+" |
            "+#" |
            "+." |
            "-" |
            "-#" |
            "-." |
            "*" |
            "*#" |
            "*." |
            "/" |
            "/#" |
            "/." |
            "mod" |
            "mod." |
            "=" |
            "=?" |
            "=#" |
            "=." |
            "<" |
            "<#" |
            "<." |
            ">" |
            ">#" |
            ">." |
            "<=" |
            "<=#" |
            "<=." |
            ">=" |
            ">=#" |
            ">=." |
            "and" |
            "or" |
            "not" |
            "~" |
            "^" |
            "|" |
            "&" |
            "<<" |
            ">>" |
            "length" |
            "get" |
            "car" |
            "cdr" |
            "fst" |
            "snd" |
            "Int->Dec" |
            "Dec->Int" |
            "as" |
            "char" |
            "vector" |
            "string" |
            "tuple" |
            "lambda" |
            "if" |
            "while" |
            "let" |
            "letrec" |
            "mut" |
            "do"
    )
}

fn local_lookup_fn_effect(
    local_fn_scopes: &[HashMap<String, EffectFlags>],
    name: &str
) -> Option<EffectFlags> {
    for scope in local_fn_scopes.iter().rev() {
        if let Some(effect) = scope.get(name) {
            return Some(*effect);
        }
    }
    None
}

fn estimate_effect_immutable(
    node: &TypedExpression,
    top_fn_effects: &HashMap<String, EffectFlags>,
    local_fn_scopes: &mut Vec<HashMap<String, EffectFlags>>
) -> EffectFlags {
    match &node.expr {
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => EffectFlags::PURE,
        Expression::Apply(items) => {
            if items.is_empty() {
                return EffectFlags::PURE;
            }

            let Some(head) = items.first() else {
                return EffectFlags::PURE;
            };
            let Some(head_child) = node.children.first() else {
                return EffectFlags::PURE;
            };

            match head {
                Expression::Word(op) if op == "lambda" => {
                    if node.children.is_empty() {
                        return EffectFlags::PURE;
                    }
                    let body_idx = node.children.len() - 1;
                    local_fn_scopes.push(HashMap::new());
                    let effect = estimate_effect_immutable(
                        &node.children[body_idx],
                        top_fn_effects,
                        local_fn_scopes
                    );
                    local_fn_scopes.pop();
                    effect
                }
                Expression::Word(op) if op == "if" => {
                    node.children
                        .iter()
                        .skip(1)
                        .fold(
                            EffectFlags::PURE,
                            |acc, ch|
                                acc | estimate_effect_immutable(ch, top_fn_effects, local_fn_scopes)
                        )
                }
                Expression::Word(op) if op == "while" => {
                    node.children
                        .iter()
                        .skip(1)
                        .fold(
                            EffectFlags::PURE,
                            |acc, ch|
                                acc | estimate_effect_immutable(ch, top_fn_effects, local_fn_scopes)
                        )
                }
                Expression::Word(op) if op == "do" => {
                    local_fn_scopes.push(HashMap::new());
                    let mut effect = EffectFlags::PURE;
                    for (idx, ch) in node.children.iter().enumerate().skip(1) {
                        effect |= estimate_effect_immutable(ch, top_fn_effects, local_fn_scopes);
                        if let Some(Expression::Apply(form_items)) = items.get(idx) {
                            if
                                let [Expression::Word(kw), Expression::Word(name), rhs] =
                                    &form_items[..]
                            {
                                if
                                    (kw == "let" || kw == "letrec") &&
                                    matches!(rhs, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                                {
                                    if let Some(rhs_node) = ch.children.get(2) {
                                        if let Some(scope) = local_fn_scopes.last_mut() {
                                            scope.insert(name.clone(), rhs_node.effect);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    local_fn_scopes.pop();
                    effect
                }
                Expression::Word(op) if op == "let" || op == "letrec" || op == "mut" => {
                    node.children
                        .get(2)
                        .map(|rhs| estimate_effect_immutable(rhs, top_fn_effects, local_fn_scopes))
                        .unwrap_or(EffectFlags::PURE)
                }
                Expression::Word(op) => {
                    let mut effect = EffectFlags::PURE;
                    for ch in node.children.iter().skip(1) {
                        effect |= estimate_effect_immutable(ch, top_fn_effects, local_fn_scopes);
                    }

                    if is_io_op(op) {
                        effect |= EffectFlags::IO;
                    } else if is_mutating_op(op) {
                        effect |= EffectFlags::MUTATE;
                    } else if let Some(local_effect) = local_lookup_fn_effect(local_fn_scopes, op) {
                        effect |= local_effect;
                    } else if let Some(top_effect) = top_fn_effects.get(op) {
                        effect |= *top_effect;
                    } else if !is_intrinsic_pure_op(op) {
                        effect |= EffectFlags::UNKNOWN_CALL;
                    }

                    effect
                }
                _ => {
                    let mut effect = EffectFlags::UNKNOWN_CALL;
                    effect |= estimate_effect_immutable(
                        head_child,
                        top_fn_effects,
                        local_fn_scopes
                    );
                    for ch in node.children.iter().skip(1) {
                        effect |= estimate_effect_immutable(ch, top_fn_effects, local_fn_scopes);
                    }
                    effect
                }
            }
        }
    }
}

fn annotate_effects_mut(
    node: &mut TypedExpression,
    top_fn_effects: &HashMap<String, EffectFlags>,
    local_fn_scopes: &mut Vec<HashMap<String, EffectFlags>>
) -> EffectFlags {
    let effect = match &node.expr {
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => EffectFlags::PURE,
        Expression::Apply(items) => {
            if items.is_empty() {
                EffectFlags::PURE
            } else {
                match items.first() {
                    Some(Expression::Word(op)) if op == "lambda" => {
                        if node.children.is_empty() {
                            EffectFlags::PURE
                        } else {
                            let body_idx = node.children.len() - 1;
                            local_fn_scopes.push(HashMap::new());
                            let body_effect = annotate_effects_mut(
                                &mut node.children[body_idx],
                                top_fn_effects,
                                local_fn_scopes
                            );
                            local_fn_scopes.pop();
                            body_effect
                        }
                    }
                    Some(Expression::Word(op)) if op == "if" || op == "while" => {
                        let mut combined = EffectFlags::PURE;
                        for ch in node.children.iter_mut().skip(1) {
                            combined |= annotate_effects_mut(ch, top_fn_effects, local_fn_scopes);
                        }
                        combined
                    }
                    Some(Expression::Word(op)) if op == "do" => {
                        local_fn_scopes.push(HashMap::new());
                        let mut combined = EffectFlags::PURE;
                        for idx in 1..node.children.len() {
                            let child_effect = annotate_effects_mut(
                                &mut node.children[idx],
                                top_fn_effects,
                                local_fn_scopes
                            );
                            combined |= child_effect;
                            if let Some(Expression::Apply(form_items)) = items.get(idx) {
                                if
                                    let [Expression::Word(kw), Expression::Word(name), rhs] =
                                        &form_items[..]
                                {
                                    if
                                        (kw == "let" || kw == "letrec") &&
                                        matches!(rhs, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                                    {
                                        if let Some(rhs_node) = node.children[idx].children.get(2) {
                                            if let Some(scope) = local_fn_scopes.last_mut() {
                                                scope.insert(name.clone(), rhs_node.effect);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        local_fn_scopes.pop();
                        combined
                    }
                    Some(Expression::Word(op)) if op == "let" || op == "letrec" || op == "mut" => {
                        node.children
                            .get_mut(2)
                            .map(|rhs| annotate_effects_mut(rhs, top_fn_effects, local_fn_scopes))
                            .unwrap_or(EffectFlags::PURE)
                    }
                    Some(Expression::Word(op)) => {
                        let mut combined = EffectFlags::PURE;
                        for ch in node.children.iter_mut().skip(1) {
                            combined |= annotate_effects_mut(ch, top_fn_effects, local_fn_scopes);
                        }

                        if is_io_op(op) {
                            combined |= EffectFlags::IO;
                        } else if is_mutating_op(op) {
                            combined |= EffectFlags::MUTATE;
                        } else if
                            let Some(local_effect) = local_lookup_fn_effect(local_fn_scopes, op)
                        {
                            combined |= local_effect;
                        } else if let Some(top_effect) = top_fn_effects.get(op) {
                            combined |= *top_effect;
                        } else if !is_intrinsic_pure_op(op) {
                            combined |= EffectFlags::UNKNOWN_CALL;
                        }

                        combined
                    }
                    _ => {
                        let mut combined = EffectFlags::UNKNOWN_CALL;
                        for ch in node.children.iter_mut() {
                            combined |= annotate_effects_mut(ch, top_fn_effects, local_fn_scopes);
                        }
                        combined
                    }
                }
            }
        }
    };
    node.effect = effect;
    effect
}

fn collect_top_level_lambda_defs<'a>(
    node: &'a TypedExpression
) -> HashMap<String, &'a TypedExpression> {
    let mut defs = HashMap::new();
    let Expression::Apply(items) = &node.expr else {
        return defs;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return defs;
    }
    for (idx, item) in items.iter().enumerate().skip(1) {
        let Expression::Apply(form_items) = item else {
            continue;
        };
        let [Expression::Word(kw), Expression::Word(name), rhs] = &form_items[..] else {
            continue;
        };
        if kw != "let" && kw != "letrec" {
            continue;
        }
        if
            !matches!(rhs, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
        {
            continue;
        }
        if let Some(node_item) = node.children.get(idx).and_then(|n| n.children.get(2)) {
            defs.insert(name.clone(), node_item);
        }
    }
    defs
}

fn compute_top_level_function_effects(root: &TypedExpression) -> HashMap<String, EffectFlags> {
    let defs = collect_top_level_lambda_defs(root);
    let mut summaries: HashMap<String, EffectFlags> = defs
        .keys()
        .cloned()
        .map(|name| (name, EffectFlags::PURE))
        .collect();
    if summaries.is_empty() {
        return summaries;
    }

    let max_iters = summaries.len().saturating_mul(4).max(4);
    for _ in 0..max_iters {
        let mut changed = false;
        for (name, lambda_node) in &defs {
            let effect = if lambda_node.children.is_empty() {
                EffectFlags::PURE
            } else {
                let body_idx = lambda_node.children.len() - 1;
                let mut scopes = vec![HashMap::new()];
                estimate_effect_immutable(&lambda_node.children[body_idx], &summaries, &mut scopes)
            };
            if summaries.get(name).copied() != Some(effect) {
                summaries.insert(name.clone(), effect);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    summaries
}

fn annotate_effects(root: &mut TypedExpression) {
    let top_effects = compute_top_level_function_effects(root);
    let mut scopes = vec![HashMap::new()];
    let _ = annotate_effects_mut(root, &top_effects, &mut scopes);
}

fn validate_impure_function_name_suffix(root: &TypedExpression) -> Result<(), String> {
    let mut known_requires_bang: HashMap<String, bool> = HashMap::new();
    if let Expression::Apply(items) = &root.expr {
        if matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
            if items.len() != root.children.len() {
                return Ok(());
            }
            for idx in 1..items.len() {
                let Some(let_node) = root.children.get(idx) else {
                    continue;
                };
                if
                    let Some(message) = check_impure_binding_name(
                        &items[idx],
                        let_node,
                        &mut known_requires_bang
                    )
                {
                    return Err(message);
                }
            }
            return Ok(());
        }
        if
            let Some(message) = check_impure_binding_name(
                &root.expr,
                root,
                &mut known_requires_bang
            )
        {
            return Err(message);
        }
    }

    Ok(())
}

fn check_impure_binding_name(
    item_expr: &Expression,
    let_node: &TypedExpression,
    known_requires_bang: &mut HashMap<String, bool>
) -> Option<String> {
    let (name, requires_bang) = eval_function_binding_requires_bang(
        item_expr,
        let_node,
        known_requires_bang
    )?;
    if requires_bang {
        if
            let Some(offending_idx) = eval_function_binding_non_first_mutation_target(
                item_expr,
                known_requires_bang
            )
        {
            return Some(
                format!(
                    "Impure function '{}' must mutate its first parameter (argument 1); found mutation target using argument {}\n{}",
                    name,
                    offending_idx + 1,
                    item_expr.to_lisp()
                )
            );
        }
    }
    if
        !requires_bang ||
        is_impure_bang_exception_name(&name) ||
        name.ends_with('!') ||
        name.starts_with('_')
    {
        return None;
    }
    Some(format!("Impure function '{}' must end with '!'\n{}", name, item_expr.to_lisp()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MutationBinding {
    Param(usize),
    Local,
}

fn resolve_mutation_binding(
    scopes: &[HashMap<String, MutationBinding>],
    name: &str
) -> Option<MutationBinding> {
    for scope in scopes.iter().rev() {
        if let Some(binding) = scope.get(name) {
            return Some(*binding);
        }
    }
    None
}

fn collect_target_param_usage(
    expr: &Expression,
    scopes: &[HashMap<String, MutationBinding>],
    uses_allowed_param: &mut bool,
    first_non_first_param: &mut Option<usize>
) {
    match expr {
        Expression::Int(_) | Expression::Dec(_) => {}
        Expression::Word(name) => {
            if let Some(MutationBinding::Param(idx)) = resolve_mutation_binding(scopes, name) {
                if idx == 0 {
                    *uses_allowed_param = true;
                } else if first_non_first_param.is_none() {
                    *first_non_first_param = Some(idx);
                }
            }
        }
        Expression::Apply(items) => {
            if items.is_empty() {
                return;
            }
            if matches!(items.first(), Some(Expression::Word(_))) {
                for item in items.iter().skip(1) {
                    collect_target_param_usage(
                        item,
                        scopes,
                        uses_allowed_param,
                        first_non_first_param
                    );
                }
            } else {
                for item in items {
                    collect_target_param_usage(
                        item,
                        scopes,
                        uses_allowed_param,
                        first_non_first_param
                    );
                }
            }
        }
    }
}

fn target_non_first_param_index(
    expr: &Expression,
    scopes: &[HashMap<String, MutationBinding>]
) -> Option<usize> {
    let mut uses_allowed_param = false;
    let mut first_non_first_param = None;
    collect_target_param_usage(expr, scopes, &mut uses_allowed_param, &mut first_non_first_param);
    if uses_allowed_param {
        None
    } else {
        first_non_first_param
    }
}

fn expr_mutates_non_first_param(
    expr: &Expression,
    known_requires_bang: &HashMap<String, bool>,
    scopes: &mut Vec<HashMap<String, MutationBinding>>
) -> Option<usize> {
    match expr {
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => None,
        Expression::Apply(items) => {
            if items.is_empty() {
                return None;
            }
            let Some(head) = items.first() else {
                return None;
            };
            match head {
                Expression::Word(op) if op == "lambda" => None,
                Expression::Word(op) if op == "do" => {
                    scopes.push(HashMap::new());
                    for item in items.iter().skip(1) {
                        if
                            let Some(idx) = expr_mutates_non_first_param(
                                item,
                                known_requires_bang,
                                scopes
                            )
                        {
                            scopes.pop();
                            return Some(idx);
                        }
                        if let Expression::Apply(form_items) = item {
                            if
                                let [Expression::Word(kw), Expression::Word(name), _rhs] =
                                    &form_items[..]
                            {
                                if kw == "let" || kw == "letrec" || kw == "mut" {
                                    if let Some(scope) = scopes.last_mut() {
                                        scope.insert(name.clone(), MutationBinding::Local);
                                    }
                                }
                            }
                        }
                    }
                    scopes.pop();
                    None
                }
                Expression::Word(op) if op == "let" || op == "letrec" || op == "mut" => {
                    items
                        .get(2)
                        .and_then(|rhs|
                            expr_mutates_non_first_param(rhs, known_requires_bang, scopes)
                        )
                }
                Expression::Word(op) if is_bang_contract_op(op, known_requires_bang) => {
                    if let Some(target) = items.get(1) {
                        if let Some(idx) = target_non_first_param_index(target, scopes) {
                            return Some(idx);
                        }
                    }
                    for item in items.iter().skip(1) {
                        if
                            let Some(idx) = expr_mutates_non_first_param(
                                item,
                                known_requires_bang,
                                scopes
                            )
                        {
                            return Some(idx);
                        }
                    }
                    None
                }
                _ => {
                    for item in items.iter().skip(1) {
                        if
                            let Some(idx) = expr_mutates_non_first_param(
                                item,
                                known_requires_bang,
                                scopes
                            )
                        {
                            return Some(idx);
                        }
                    }
                    None
                }
            }
        }
    }
}

fn eval_function_binding_non_first_mutation_target(
    item_expr: &Expression,
    known_requires_bang: &HashMap<String, bool>
) -> Option<usize> {
    let Expression::Apply(let_items) = item_expr else {
        return None;
    };
    let [Expression::Word(keyword), Expression::Word(_name), rhs] = &let_items[..] else {
        return None;
    };
    if keyword != "let" && keyword != "letrec" && keyword != "mut" {
        return None;
    }
    let Expression::Apply(lambda_items) = rhs else {
        return None;
    };
    if
        lambda_items.len() < 2 ||
        !matches!(lambda_items.first(), Some(Expression::Word(w)) if w == "lambda")
    {
        return None;
    }
    let Some(body) = lambda_items.last() else {
        return None;
    };

    let mut scopes: Vec<HashMap<String, MutationBinding>> = vec![HashMap::new()];
    if let Some(scope) = scopes.last_mut() {
        for (idx, param) in lambda_items
            .iter()
            .skip(1)
            .take(lambda_items.len().saturating_sub(2))
            .enumerate() {
            if let Expression::Word(name) = param {
                scope.insert(name.clone(), MutationBinding::Param(idx));
            }
        }
    }
    expr_mutates_non_first_param(body, known_requires_bang, &mut scopes)
}

fn is_impure_bang_exception_name(name: &str) -> bool {
    matches!(
        name,
        "set" |
            "Id" |
            "+=" |
            "-=" |
            "*=" |
            "/=" |
            "++" |
            "--" |
            "**" |
            "+=." |
            "-=." |
            "*=." |
            "/=." |
            "++." |
            "--." |
            "**."
    )
}

fn eval_function_binding_requires_bang(
    item_expr: &Expression,
    let_node: &TypedExpression,
    known_requires_bang: &mut HashMap<String, bool>
) -> Option<(String, bool)> {
    let Expression::Apply(let_items) = item_expr else {
        return None;
    };
    let [Expression::Word(keyword), Expression::Word(name), rhs] = &let_items[..] else {
        return None;
    };
    if keyword != "let" && keyword != "letrec" {
        return None;
    }

    let rhs_node = let_node.children.get(2)?;
    let is_function_binding = matches!(rhs_node.typ, Some(Type::Function(_, _)));
    if !is_function_binding {
        return None;
    }

    let requires_bang = match rhs {
        Expression::Word(alias_target) => {
            if alias_target.contains('/') {
                false
            } else {
                known_requires_bang.get(alias_target).copied().unwrap_or(false)
            }
        }
        _ =>
            match rhs {
                Expression::Apply(rhs_items) =>
                    matches!(rhs_items.first(), Some(Expression::Word(w)) if w == "lambda") &&
                        lambda_requires_bang(rhs, known_requires_bang),
                _ => false,
            }
    };

    known_requires_bang.insert(name.clone(), requires_bang);
    Some((name.clone(), requires_bang))
}

pub fn collect_top_level_function_external_impurity(
    root: &TypedExpression,
    out: &mut HashMap<String, bool>
) {
    let mut known_requires_bang: HashMap<String, bool> = HashMap::new();
    if let Expression::Apply(items) = &root.expr {
        if matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
            if items.len() == root.children.len() {
                for idx in 1..items.len() {
                    let Some(let_node) = root.children.get(idx) else {
                        continue;
                    };
                    if
                        let Some((name, requires)) = eval_function_binding_requires_bang(
                            &items[idx],
                            let_node,
                            &mut known_requires_bang
                        )
                    {
                        out.insert(name, requires);
                    }
                }
            }
            return;
        }
        if
            let Some((name, requires)) = eval_function_binding_requires_bang(
                &root.expr,
                root,
                &mut known_requires_bang
            )
        {
            out.insert(name, requires);
        }
    }
}

fn lambda_requires_bang(
    lambda_expr: &Expression,
    known_requires_bang: &HashMap<String, bool>
) -> bool {
    let Expression::Apply(items) = lambda_expr else {
        return false;
    };
    if items.len() < 2 || !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
        return false;
    }

    let mut scopes: Vec<HashMap<String, bool>> = vec![HashMap::new()];
    if let Some(scope) = scopes.last_mut() {
        for param in &items[1..items.len().saturating_sub(1)] {
            if let Expression::Word(name) = param {
                scope.insert(name.clone(), true);
            }
        }
    }
    if let Some(body) = items.last() {
        expr_requires_bang(body, known_requires_bang, &mut scopes)
    } else {
        false
    }
}

fn resolve_binding_kind(scopes: &[HashMap<String, bool>], name: &str) -> Option<bool> {
    for scope in scopes.iter().rev() {
        if let Some(kind) = scope.get(name) {
            return Some(*kind);
        }
    }
    None
}

fn expr_requires_bang(
    expr: &Expression,
    known_requires_bang: &HashMap<String, bool>,
    scopes: &mut Vec<HashMap<String, bool>>
) -> bool {
    match expr {
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => false,
        Expression::Apply(items) => {
            if items.is_empty() {
                return false;
            }
            let Some(head) = items.first() else {
                return false;
            };
            match head {
                Expression::Word(op) if op == "lambda" => {
                    // Nested lambda effects are checked when that lambda itself is bound.
                    false
                }
                Expression::Word(op) if op == "do" => {
                    scopes.push(HashMap::new());
                    for item in items.iter().skip(1) {
                        if expr_requires_bang(item, known_requires_bang, scopes) {
                            scopes.pop();
                            return true;
                        }
                        if let Expression::Apply(form_items) = item {
                            if
                                let [Expression::Word(kw), Expression::Word(name), _rhs] =
                                    &form_items[..]
                            {
                                if kw == "let" || kw == "letrec" || kw == "mut" {
                                    if let Some(scope) = scopes.last_mut() {
                                        scope.insert(name.clone(), false);
                                    }
                                }
                            }
                        }
                    }
                    scopes.pop();
                    false
                }
                Expression::Word(op) if op == "let" || op == "letrec" || op == "mut" => {
                    items
                        .get(2)
                        .map(|rhs| expr_requires_bang(rhs, known_requires_bang, scopes))
                        .unwrap_or(false)
                }
                Expression::Word(op) if is_io_op(op) => true,
                Expression::Word(op) if is_bang_contract_op(op, known_requires_bang) => {
                    let Some(target) = items.get(1) else {
                        return true;
                    };
                    match target {
                        Expression::Word(name) => {
                            if let Some(is_param) = resolve_binding_kind(scopes, name) {
                                is_param
                            } else {
                                true
                            }
                        }
                        _ => expr_uses_param_or_free_var(target, scopes),
                    }
                }
                Expression::Word(_op) => {
                    for arg in items.iter().skip(1) {
                        if expr_requires_bang(arg, known_requires_bang, scopes) {
                            return true;
                        }
                    }
                    false
                }
                _ => {
                    for it in items {
                        if expr_requires_bang(it, known_requires_bang, scopes) {
                            return true;
                        }
                    }
                    false
                }
            }
        }
    }
}

fn expr_uses_param_or_free_var(expr: &Expression, scopes: &[HashMap<String, bool>]) -> bool {
    match expr {
        Expression::Int(_) | Expression::Dec(_) => false,
        Expression::Word(name) => {
            if let Some(is_param) = resolve_binding_kind(scopes, name) { is_param } else { true }
        }
        Expression::Apply(items) => {
            if items.is_empty() {
                return false;
            }
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda") {
                return false;
            }
            if matches!(items.first(), Some(Expression::Word(_))) {
                items
                    .iter()
                    .skip(1)
                    .any(|item| expr_uses_param_or_free_var(item, scopes))
            } else {
                items.iter().any(|item| expr_uses_param_or_free_var(item, scopes))
            }
        }
    }
}

fn src_to_pretty(src: &TypeError) -> String {
    let joined = src.expr
        .iter()
        .map(|e| e.to_lisp())
        .collect::<Vec<_>>()
        .join(" ");
    match src.variant {
        TypeErrorVariant::Vector => format!("(vector {})", joined),
        TypeErrorVariant::Call => format!("({})", joined),
        TypeErrorVariant::IfCond => format!("Condition must be Bool\n(if {})", joined),
        TypeErrorVariant::IfBody => {
            format!("Concequent and alternative must match types\n(if {})", joined)
        }
        TypeErrorVariant::Source => joined,
    }
}

fn with_src(message: String, src: &TypeError) -> String {
    let snippet = src_to_pretty(src);
    if snippet.trim().is_empty() {
        message
    } else {
        format!("{}\n{}", message, snippet)
    }
}

#[derive(Clone, Debug)]
pub struct SolveError {
    pub message: String,
    pub scope: Option<InferErrorScope>,
}

pub struct InferenceContext {
    pub env: TypeEnv,
    pub mut_scopes: Vec<HashSet<String>>,
    pub lambda_scope_bases: Vec<usize>,
    pub constraints: Vec<(Type, Type, TypeError)>,
    pub fresh_var_counter: u64,
    pub expr_types: HashMap<usize, Type>,
    pub collect_expr_types: bool,
    pub root_expr_id: usize,
    pub user_form_count_for_scope: Option<usize>,
    pub current_user_top_form: Option<usize>,
    pub scope_lambda_path: Vec<usize>,
    pub scope_lambda_counters: Vec<usize>,
    pub last_error_scope: Option<InferErrorScope>,
}

impl InferenceContext {
    pub fn add_constraint(&mut self, t1: Type, t2: Type, src: TypeError) {
        self.constraints.push((t1, t2, src));
    }

    pub fn fresh_var(&mut self) -> Type {
        let var = TypeVar::new(self.fresh_var_counter);
        self.fresh_var_counter += 1;
        Type::Var(var)
    }

    pub fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        use std::collections::HashMap;
        let mut mapping: HashMap<u64, Type> = HashMap::new();
        for id in &scheme.vars {
            mapping.insert(*id, self.fresh_var());
        }
        scheme.typ.substitute(&mapping)
    }

    pub fn current_error_scope(&self) -> Option<InferErrorScope> {
        self.current_user_top_form.map(|user_top_form| InferErrorScope {
            user_top_form,
            lambda_path: self.scope_lambda_path.clone(),
        })
    }

    pub fn type_error(&self, variant: TypeErrorVariant, expr: Vec<Expression>) -> TypeError {
        TypeError {
            variant,
            expr,
            scope: self.current_error_scope(),
        }
    }

    pub fn enter_user_top_form(&mut self, idx: usize) {
        self.current_user_top_form = Some(idx);
        self.scope_lambda_path.clear();
        self.scope_lambda_counters.clear();
        self.scope_lambda_counters.push(0);
    }

    pub fn leave_user_top_form(&mut self) {
        self.current_user_top_form = None;
        self.scope_lambda_path.clear();
        self.scope_lambda_counters.clear();
        self.scope_lambda_counters.push(0);
    }

    pub fn enter_lambda_scope(&mut self) {
        if self.current_user_top_form.is_none() {
            return;
        }
        if self.scope_lambda_counters.is_empty() {
            self.scope_lambda_counters.push(0);
        }
        let depth = self.scope_lambda_counters.len() - 1;
        let lambda_idx = self.scope_lambda_counters[depth];
        self.scope_lambda_counters[depth] += 1;
        self.scope_lambda_path.push(lambda_idx);
        self.scope_lambda_counters.push(0);
    }

    pub fn exit_lambda_scope(&mut self) {
        if self.current_user_top_form.is_none() {
            return;
        }
        if self.scope_lambda_counters.len() > 1 {
            self.scope_lambda_counters.pop();
        }
        self.scope_lambda_path.pop();
    }

    pub fn enter_lexical_scope(&mut self) {
        self.env.enter_scope();
        self.mut_scopes.push(HashSet::new());
    }

    pub fn exit_lexical_scope(&mut self) {
        self.env.exit_scope();
        if self.mut_scopes.len() > 1 {
            self.mut_scopes.pop();
        }
    }

    pub fn mark_mut_binding(&mut self, name: String) {
        if let Some(scope) = self.mut_scopes.last_mut() {
            scope.insert(name);
        }
    }

    pub fn is_mut_binding(&self, name: &str) -> bool {
        for (scope_idx, scope) in self.env.scopes.iter().enumerate().rev() {
            if scope.contains_key(name) {
                return self.mut_scopes
                    .get(scope_idx)
                    .map(|mut_scope| mut_scope.contains(name))
                    .unwrap_or(false);
            }
        }
        false
    }

    pub fn binding_scope_index(&self, name: &str) -> Option<usize> {
        for (scope_idx, scope) in self.env.scopes.iter().enumerate().rev() {
            if scope.contains_key(name) {
                return Some(scope_idx);
            }
        }
        None
    }

    pub fn is_mut_capture_in_lambda(&self, name: &str) -> bool {
        let Some(lambda_scope_base) = self.lambda_scope_bases.last().copied() else {
            return false;
        };
        let Some(binding_scope_idx) = self.binding_scope_index(name) else {
            return false;
        };
        if binding_scope_idx >= lambda_scope_base {
            return false;
        }
        self.mut_scopes
            .get(binding_scope_idx)
            .map(|scope| scope.contains(name))
            .unwrap_or(false)
    }
}

fn mut_capture_error(name: &str) -> String {
    format!("mut variable '{}' cannot be captured by lambda; use &mut cells for closure-shared mutation", name)
}

fn infer_expr(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    let inferred = match expr {
        Expression::Int(_) => Ok(Type::Int),
        Expression::Dec(_) => Ok(Type::Dec),

        Expression::Word(name) => {
            if ctx.is_mut_capture_in_lambda(name) {
                return Err(mut_capture_error(name));
            }
            if let Some(scheme) = ctx.env.get(name) {
                Ok(ctx.instantiate(&scheme))
            } else {
                Err(format!("Undefined variable: {}", name))
            }
        }

        Expression::Apply(exprs) => {
            if exprs.is_empty() {
                return Err("Error!: Empty application".to_string());
            }

            if let Expression::Word(func_name) = &exprs[0] {
                match func_name.as_str() {
                    "as" => infer_as(exprs, ctx),
                    "lambda" => infer_lambda(exprs, ctx),
                    "if" => infer_if(&exprs, ctx),
                    "while" => infer_while(&exprs, ctx),
                    "let" => infer_let(&exprs, ctx),
                    "mut" => infer_mut(&exprs, ctx),
                    "letrec" => infer_rec(&exprs, ctx),
                    "alter!" => infer_alter(&exprs, ctx),
                    "do" => infer_do(expr, &exprs, ctx),
                    _ => infer_function_call(exprs, ctx),
                }
            } else {
                infer_function_call(exprs, ctx)
            }
        }
    };

    if inferred.is_err() && ctx.last_error_scope.is_none() {
        ctx.last_error_scope = ctx.current_error_scope();
    }

    if ctx.collect_expr_types {
        if let Ok(typ) = &inferred {
            ctx.expr_types.insert(expression_id(expr), typ.clone());
        }
    }

    inferred
}

fn parse_type_hint(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    match expr {
        Expression::Word(name) =>
            match name.as_str() {
                "Int" => Ok(Type::Int),
                "Dec" => Ok(Type::Dec),
                "Bool" => Ok(Type::Bool),
                "Char" => Ok(Type::Char),
                _ => Ok(ctx.fresh_var()), // unknown type name
            }

        // Handles list-like hints like [Int], [[Char]], etc.
        Expression::Apply(items) if !items.is_empty() => {
            // A shorthand for [T] means (vector T)
            if let Expression::Word(t) = &items[0] {
                if t == "vector" || t == "string" {
                    if items.len() == 2 {
                        let inner = parse_type_hint(&items[1], ctx)?;
                        return Ok(Type::List(Box::new(inner)));
                    }
                } else if t == "tuple" {
                    if items.len() < 2 {
                        return Err(
                            format!("Tuple type must have at least one element: {}", expr.to_lisp())
                        );
                    }
                    let mut elems = Vec::new();
                    for elem_expr in &items[1..] {
                        elems.push(parse_type_hint(elem_expr, ctx)?);
                    }
                    return Ok(Type::Tuple(elems));
                }
            }
            Err(format!("Invalid type hint syntax: {}", expr.to_lisp()))
        }

        _ => Err(format!("Invalid type hint: {}", expr.to_lisp())),
    }
}
// arity depth (number of list nestings)
fn type_arity(t: &Type) -> usize {
    match t {
        Type::List(inner) => 1 + type_arity(inner),
        _ => 0,
    }
}
fn inner_type(t: &Type) -> &Type {
    match t {
        Type::List(inner) => inner_type(inner),
        _ => t,
    }
}
// get deepest inner type
fn deepest_type(t: &Type) -> &Type {
    match t {
        Type::List(inner) => deepest_type(inner),
        _ => t,
    }
}

pub fn infer_as(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err("as expects exactly two arguments: (as expr Type)".to_string());
    }

    // Infer both sides
    let expr_type = infer_expr(&args[0], ctx)?;
    let type_hint = parse_type_hint(&args[1], ctx)?;

    // Handle tuple special case directly — before arity logic
    match (&expr_type, &type_hint) {
        (Type::Tuple(expr_elems), Type::Tuple(hint_elems)) => {
            if expr_elems.len() != hint_elems.len() {
                return Err(
                    format!(
                        "Tuple length mismatch in as: {} vs {}\n(as {})",
                        expr_elems.len(),
                        hint_elems.len(),
                        args
                            .iter()
                            .map(|e| e.to_lisp())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                );
            }

            // Create constraints for each element
            for (e, h) in expr_elems.iter().zip(hint_elems.iter()) {
                ctx.add_constraint(
                    e.clone(),
                    h.clone(),
                    ctx.type_error(TypeErrorVariant::Source, args.to_vec())
                );
            }

            return Ok(type_hint);
        }
        (Type::Tuple(_), _) | (_, Type::Tuple(_)) => {
            return Err(
                format!(
                    "Cannot cast between tuple and non-tuple types\n(as {})",
                    args
                        .iter()
                        .map(|e| e.to_lisp())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            );
        }
        _ => {}
    }

    // Compute arities
    let expr_arity = type_arity(&expr_type);
    let hint_arity = type_arity(&type_hint);
    let inner_expr_type = deepest_type(&expr_type);
    let is_expr_var = matches!(inner_expr_type, Type::Var(_));

    // If expr_type is a type variable, allow up to (≤) right-side arity
    if is_expr_var && expr_arity > hint_arity {
        return Err(
            format!(
                "Type variable in as cannot represent deeper nesting: {} vs {}",
                expr_type,
                type_hint
            )
        );
    }

    // Check arity mismatch for lists/functions
    if !is_expr_var && expr_arity != hint_arity {
        return Err(
            format!(
                "Type arity mismatch in as: left has arity {}, right has arity {} ({} vs {})\n(as {})",
                expr_arity,
                hint_arity,
                expr_type,
                type_hint,
                args
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        );
    }

    // Array element restriction)
    if expr_arity > 0 {
        let inner_expr = inner_type(&expr_type);
        let inner_hint = inner_type(&type_hint);

        match (inner_expr, inner_hint) {
            | (Type::Int, Type::Int)
            | (Type::Dec, Type::Dec)
            | (Type::Int, Type::Bool)
            | (Type::Int, Type::Char)
            | (Type::Bool, Type::Int)
            | (Type::Bool, Type::Bool)
            | (Type::Bool, Type::Char)
            | (Type::Char, Type::Int)
            | (Type::Char, Type::Bool)
            | (Type::Char, Type::Char)
            | (Type::Var(_), _)
            | (_, Type::Var(_)) => (),
            _ => {
                return Err(
                    format!(
                        "Invalid array cast in as: cannot cast {} to {}\n(as {})",
                        expr_type,
                        type_hint,
                        args
                            .iter()
                            .map(|e| e.to_lisp())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                );
            }
        }
    }

    Ok(type_hint)
}

// Type inference for lambda expressions
fn infer_lambda(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    ctx.enter_lambda_scope();
    let result = infer_lambda_inner(exprs, ctx);
    ctx.exit_lambda_scope();
    result
}

fn infer_lambda_inner(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.is_empty() {
        return Err("Lambda requires a body".to_string());
    }

    let param_count = args.len() - 1;
    let body = &args[param_count];

    // Extract parameter names
    let mut param_names = Vec::new();
    for i in 0..param_count {
        if let Expression::Word(name) = &args[i] {
            param_names.push(name.clone());
        } else {
            return Err(
                format!(
                    "Lambda parameters must be variable names\n({})",
                    exprs
                        .iter()
                        .map(|e| e.to_lisp())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            );
        }
    }

    // Create fresh type vars
    let mut param_types = Vec::new();
    for _ in 0..param_count {
        param_types.push(ctx.fresh_var());
    }

    // Track lexical scope base for this lambda, then enter its parameter/body scope.
    let lambda_scope_base = ctx.env.scopes.len();
    ctx.lambda_scope_bases.push(lambda_scope_base);
    ctx.enter_lexical_scope();
    let body_result = (|| -> Result<Type, String> {
        // Insert parameters
        for (name, typ) in param_names.iter().zip(param_types.iter()) {
            ctx.env
                .insert(name.clone(), TypeScheme::monotype(typ.clone()))
                .map_err(|e| format!("in lambda: {}", e))?;
        }

        // Infer body type
        infer_expr(body, ctx)
    })();
    ctx.exit_lexical_scope();
    ctx.lambda_scope_bases.pop();
    let body_type = body_result?;

    // Build function type
    let func_type = if param_types.is_empty() {
        // zero-arg lambdas get an explicit () -> body_type
        Type::Function(Box::new(Type::Unit), Box::new(body_type))
    } else {
        let mut ft = body_type;
        for param_type in param_types.iter().rev() {
            ft = Type::Function(Box::new(param_type.clone()), Box::new(ft));
        }
        ft
    };

    Ok(func_type)
}

// Type inference for if expressions
fn infer_if(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args: &[Expression] = &exprs[1..];
    if args.len() != 3 {
        return Err("If requires exactly 3 arguments: condition, then, else".to_string());
    }

    let condition = &args[0];
    let then_expr = &args[1];
    let else_expr = &args[2];

    // Infer condition type - should be Bool
    let cond_type = infer_expr(condition, ctx)?;
    ctx.add_constraint(
        cond_type.clone(),
        Type::Bool,
        ctx.type_error(TypeErrorVariant::IfCond, args.to_vec())
    );
    // Infer then and else types
    let then_type = infer_expr(then_expr, ctx)?;
    let else_type = infer_expr(else_expr, ctx)?;

    // Both branches must have the same type
    ctx.add_constraint(
        then_type.clone(),
        else_type,
        ctx.type_error(TypeErrorVariant::IfBody, args.to_vec())
    );

    Ok(then_type)
}

fn infer_while(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(
            format!(
                "while expects exactly 2 arguments: condition and body\n{}",
                format!(
                    "({})",
                    exprs
                        .iter()
                        .map(|e| e.to_lisp())
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            )
        );
    }

    let cond_type = infer_expr(&args[0], ctx)?;
    ctx.add_constraint(
        cond_type,
        Type::Bool,
        ctx.type_error(TypeErrorVariant::Source, vec![args[0].clone()])
    );

    // while body is lexical: declarations inside it should not leak into surrounding scope.
    ctx.enter_lexical_scope();
    let body_result = infer_expr(&args[1], ctx);
    ctx.exit_lexical_scope();
    let body_type = body_result?;
    ctx.add_constraint(
        body_type,
        Type::Unit,
        ctx.type_error(TypeErrorVariant::Source, vec![args[1].clone()])
    );

    Ok(Type::Unit)
}

fn is_nonexpansive(expr: &Expression) -> bool {
    match expr {
        Expression::Word(_) | Expression::Int(_) | Expression::Dec(_) => true,

        Expression::Apply(list) if !list.is_empty() =>
            match &list[0] {
                Expression::Word(name) if name == "lambda" => true,
                // This is commented out because it will otherwise cause a bug with mutaiton (set!) inference
                // and keep the vector polymorphic for empty nested vectors [[]]
                // Expression::Word(name) if name == "vector" => !list[1..].is_empty(),
                _ => false,
            }

        _ => false,
    }
}

/// Unifier: mutable map from type variable id -> Type (the binding).
#[derive(Debug, Default)]
pub struct Unifier {
    binds: HashMap<u64, Type>,
}

impl Unifier {
    pub fn new() -> Self {
        Self {
            binds: HashMap::new(),
        }
    }

    // Find representative for a Type::Var(id). If bound, follow the binding
    fn find_var(&mut self, id: u64) -> Type {
        match self.binds.get(&id).cloned() {
            None => Type::Var(TypeVar::new(id)),
            Some(ty) =>
                match ty {
                    Type::Var(ref v) if v.id != id => {
                        // path compress
                        let rep = self.find_var(v.id);
                        // store the rep
                        self.binds.insert(id, rep.clone());
                        rep
                    }
                    other => other,
                }
        }
    }

    // Apply current bindings to a type (non-destructive)
    pub fn apply(&mut self, t: &Type) -> Type {
        match t {
            Type::Var(v) => {
                let rep = self.find_var(v.id);
                match rep {
                    Type::Var(_) => rep,
                    _ => self.apply(&rep),
                }
            }
            Type::List(inner) => Type::List(Box::new(self.apply(inner))),
            Type::Function(a, b) => {
                Type::Function(Box::new(self.apply(a)), Box::new(self.apply(b)))
            }
            Type::Tuple(items) =>
                Type::Tuple(
                    items
                        .iter()
                        .map(|t| self.apply(t))
                        .collect()
                ),
            other => other.clone(),
        }
    }

    // Occurs check
    fn occurs(&mut self, var_id: u64, ty: &Type) -> bool {
        match ty {
            Type::Var(v) => {
                if v.id == var_id {
                    return true;
                }
                match self.find_var(v.id) {
                    Type::Var(found) if found.id == v.id => false,
                    t => self.occurs(var_id, &t),
                }
            }
            Type::List(inner) => self.occurs(var_id, inner),
            Type::Function(a, b) => self.occurs(var_id, a) || self.occurs(var_id, b),
            Type::Tuple(items) => items.iter().any(|it| self.occurs(var_id, it)),
            _ => false,
        }
    }

    // Bind var -> type with occurs check
    fn bind_var(&mut self, var_id: u64, ty: Type) -> Result<(), String> {
        if let Type::Var(v) = &ty {
            if v.id == var_id {
                return Ok(());
            }
        }
        if self.occurs(var_id, &ty) {
            return Err(format!("Occurs check failed: t{} occurs in {}", var_id, ty));
        }
        self.binds.insert(var_id, ty);
        Ok(())
    }

    // Turn the internal binds into a fully-applied substitution map
    pub fn into_substitution(mut self) -> HashMap<u64, Type> {
        // Ensure bindings are normalized (apply recursively)
        let keys: Vec<u64> = self.binds.keys().cloned().collect();
        for k in keys {
            if let Some(ty) = self.binds.get(&k).cloned() {
                let applied = {
                    // create a small temporary unifier to apply recursively (or re-use self.apply)
                    // we can call self.apply(&ty) but that mutates via path compression, that's fine
                    self.apply(&ty)
                };
                self.binds.insert(k, applied);
            }
        }
        self.binds
    }
}

/// Solve constraints: each constraint carries a TypeError (source) so we can produce a helpful message.
pub fn solve_constraints_list(
    constraints: &Vec<(Type, Type, TypeError)>
) -> Result<HashMap<u64, Type>, SolveError> {
    let mut unifier = Unifier::new();
    let mut work: VecDeque<(Type, Type, TypeError)> = VecDeque::new();

    for (a, b, src) in constraints.iter() {
        work.push_back((a.clone(), b.clone(), src.clone()));
    }

    while let Some((left, right, src)) = work.pop_front() {
        let left_ap = unifier.apply(&left);
        let right_ap = unifier.apply(&right);

        match (left_ap.clone(), right_ap.clone()) {
            (a2, b2) if a2 == b2 => {} // ok

            (Type::Var(v), ty) | (ty, Type::Var(v)) => {
                if let Err(e) = unifier.bind_var(v.id, ty) {
                    // attach source info and return
                    return Err(SolveError {
                        message: with_src(e, &src),
                        scope: src.scope.clone(),
                    });
                }
            }
            (Type::List(a_inner), Type::List(b_inner)) => {
                work.push_back((*a_inner, *b_inner, src));
            }
            (Type::Function(a1, a2), Type::Function(b1, b2)) => {
                work.push_back((*a1, *b1, src.clone()));
                work.push_back((*a2, *b2, src));
            }
            (Type::Tuple(a_items), Type::Tuple(b_items)) => {
                if a_items.len() != b_items.len() {
                    return Err(SolveError {
                        message: with_src(
                            format!(
                                "Cannot unify tuples of different lengths ({} vs {})",
                                a_items.len(),
                                b_items.len()
                            ),
                            &src
                        ),
                        scope: src.scope.clone(),
                    });
                }
                for (ai, bi) in a_items.into_iter().zip(b_items.into_iter()) {
                    work.push_back((ai, bi, src.clone()));
                }
            }
            (a2, b2) => {
                // can't unify, attach source and return
                return Err(SolveError {
                    message: with_src(format!("Cannot unify {} with {}", a2, b2), &src),
                    scope: src.scope.clone(),
                });
            }
        }
    }

    Ok(unifier.into_substitution())
}

pub fn apply_subst_map_to_type(subst: &HashMap<u64, Type>, ty: &Type) -> Type {
    match ty {
        Type::Var(var) =>
            match subst.get(&var.id) {
                Some(t) => apply_subst_map_to_type(subst, t),
                None => Type::Var(var.clone()),
            }
        Type::List(inner) => Type::List(Box::new(apply_subst_map_to_type(subst, inner))),
        Type::Function(a, b) =>
            Type::Function(
                Box::new(apply_subst_map_to_type(subst, a)),
                Box::new(apply_subst_map_to_type(subst, b))
            ),
        Type::Tuple(items) =>
            Type::Tuple(
                items
                    .iter()
                    .map(|it| apply_subst_map_to_type(subst, it))
                    .collect()
            ),
        other => other.clone(),
    }
}
fn infer_rec(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(
            format!(
                "Let requires exactly 2 arguments: variable and value\n({})",
                exprs
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        );
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    if let Expression::Word(var_name) = var_expr {
        let name = var_name.to_string();

        // assign a fresh monotype placeholder
        let tv = ctx.fresh_var();

        ctx.env.insert(name.clone(), TypeScheme::monotype(tv.clone()))?;

        let value_type = infer_expr(value_expr, ctx)?;

        // solve constraints
        let constraints_vec = ctx.constraints.clone();
        let subst_map = solve_constraints_list(&constraints_vec).map_err(|e| {
            ctx.last_error_scope = e.scope;
            e.message
        })?;
        let solved_type = apply_subst_map_to_type(&subst_map, &value_type);
        ctx.env.apply_substitution_map(&subst_map);

        // generalize only if nonexpansive
        if is_nonexpansive(value_expr) {
            generalize(&ctx.env, solved_type);
        } else {
            return Err("Only recursive functions allowed for letrec optimization".to_string());
        }

        Ok(Type::Unit)
    } else {
        Err("Let variable must be a variable name".to_string())
    }
}

fn infer_let(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(
            format!(
                "Let requires exactly 2 arguments: variable and value\n({})",
                exprs
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        );
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    if let Expression::Word(var_name) = var_expr {
        let value_type = infer_expr(value_expr, ctx)?;

        let constraints_vec: Vec<(Type, Type, TypeError)> = ctx.constraints
            .iter()
            .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
            .collect();

        let subst_map = solve_constraints_list(&constraints_vec).map_err(|e| {
            ctx.last_error_scope = e.scope;
            e.message
        })?;

        let solved_type = apply_subst_map_to_type(&subst_map, &value_type);
        ctx.env.apply_substitution_map(&subst_map);

        // Apply value restriction
        let scheme = if is_nonexpansive(value_expr) {
            generalize(&ctx.env, solved_type)
        } else {
            TypeScheme::monotype(solved_type)
        };

        ctx.env.insert(var_name.clone(), scheme)?;
        Ok(Type::Unit)
    } else {
        Err("Let variable must be a variable name".to_string())
    }
}

fn infer_mut(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(
            format!(
                "mut requires exactly 2 arguments: variable and value\n({})",
                exprs
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        );
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    if let Expression::Word(var_name) = var_expr {
        let value_type = infer_expr(value_expr, ctx)?;

        let constraints_vec: Vec<(Type, Type, TypeError)> = ctx.constraints
            .iter()
            .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
            .collect();

        let subst_map = solve_constraints_list(&constraints_vec).map_err(|e| {
            ctx.last_error_scope = e.scope;
            e.message
        })?;

        let solved_type = apply_subst_map_to_type(&subst_map, &value_type);
        ctx.env.apply_substitution_map(&subst_map);

        if matches!(solved_type, Type::Function(_, _)) {
            return Err("mut cannot bind function values".to_string());
        }
        if matches!(solved_type, Type::List(_)) {
            return Err("mut cannot bind vector values".to_string());
        }
        if matches!(solved_type, Type::Tuple(_)) {
            return Err("mut cannot bind tuple values".to_string());
        }
        if matches!(solved_type, Type::Unit) {
            return Err("mut cannot bind Unit values".to_string());
        }
        // Mutable bindings are monomorphic by design.
        ctx.env.insert(var_name.clone(), TypeScheme::monotype(solved_type))?;
        ctx.mark_mut_binding(var_name.clone());
        Ok(Type::Unit)
    } else {
        Err("mut variable must be a variable name".to_string())
    }
}

fn infer_alter(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(
            format!(
                "alter! requires exactly 2 arguments: variable and value\n({})",
                exprs
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        );
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    let var_name = match var_expr {
        Expression::Word(name) => name,
        _ => {
            return Err("alter! first argument must be a mutable variable name".to_string());
        }
    };

    if ctx.is_mut_capture_in_lambda(var_name) {
        return Err(mut_capture_error(var_name));
    }

    let Some(var_scheme) = ctx.env.get(var_name) else {
        return Err(format!("Undefined variable: {}", var_name));
    };

    if !ctx.is_mut_binding(var_name) {
        return Err(format!("alter! can only update mutable variables: {}", var_name));
    }

    let var_type = ctx.instantiate(&var_scheme);
    let value_type = infer_expr(value_expr, ctx)?;
    ctx.add_constraint(
        var_type,
        value_type,
        ctx.type_error(TypeErrorVariant::Source, args.to_vec())
    );
    Ok(Type::Unit)
}

// Type inference for do expressions
fn infer_do(
    expr: &Expression,
    exprs: &[Expression],
    ctx: &mut InferenceContext
) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.is_empty() {
        return Err("do requires at least one expression".to_string());
    }

    let mut last_type = Type::Unit; // Default type
    let is_root_do = expression_id(expr) == ctx.root_expr_id;
    let user_start = if is_root_do {
        let user_form_count = ctx.user_form_count_for_scope.unwrap_or(0);
        args.len().saturating_sub(user_form_count)
    } else {
        0
    };

    for (idx, item) in args.iter().enumerate() {
        if is_root_do {
            if idx >= user_start {
                ctx.enter_user_top_form(idx - user_start);
            } else {
                ctx.leave_user_top_form();
            }
        }

        last_type = infer_expr(item, ctx)?;
    }

    if is_root_do {
        ctx.leave_user_top_form();
    }

    Ok(last_type)
}
//
// Type inference for function calls
fn infer_function_call(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if exprs.is_empty() {
        return Err("Function call requires at least a function".to_string());
    }

    // Special handling for vector before anything else
    if let Expression::Word(name) = &exprs[0] {
        if name == "vector" {
            let args = &exprs[1..];
            if args.is_empty() {
                return Ok(Type::List(Box::new(ctx.fresh_var())));
            }

            let mut elem_types = Vec::new();
            for arg in args {
                match infer_expr(arg, ctx) {
                    Ok(elem_type) => {
                        elem_types.push(elem_type);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            let first = elem_types[0].clone();
            for t in &elem_types[1..] {
                ctx.add_constraint(
                    first.clone(),
                    t.clone(),
                    ctx.type_error(TypeErrorVariant::Vector, args.to_vec())
                );
            }

            // Return the type of the vector (List of the first element type)
            return Ok(Type::List(Box::new(first)));
        } else if name == "string" {
            let args = &exprs[1..];
            if args.is_empty() {
                return Ok(Type::List(Box::new(Type::Char))); // empty string
            }
            // We will not check if elements in string are the same
            // They should be because string is not really used by the user
            // but by the parser when transforming double quotes
            // for arg in args {
            //     match infer_expr(arg, ctx) {
            //         Ok(elem_type) => {
            //             let valid_type = match elem_type {
            //                 Type::Char => Type::Char,
            //                 Type::Int => Type::Char,
            //                 _ => elem_type,
            //             };
            //             ctx.add_constraint(
            //                 Type::Char,
            //                 valid_type,
            //                 TypeError {
            //                     variant: TypeErrorVariant::Vector,
            //                     expr: args.to_vec(),
            //                 },
            //             );
            //         }
            //         Err(e) => return Err(e),
            //     }
            // }

            return Ok(Type::List(Box::new(Type::Char)));
        } else if name == "char" {
            let args = &exprs[1..];
            if args.is_empty() {
                return Ok(Type::Char);
            }

            for arg in args {
                match infer_expr(arg, ctx) {
                    Ok(elem_type) => {
                        let valid_type = match elem_type {
                            Type::Char => Type::Char,
                            Type::Int => Type::Char,
                            _ => elem_type,
                        };
                        ctx.add_constraint(
                            Type::Char,
                            valid_type,
                            ctx.type_error(TypeErrorVariant::Vector, args.to_vec())
                        );
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            return Ok(Type::Char);
        } else if name == "tuple" {
            let args = &exprs[1..];
            if args.len() != 2 {
                return Err(
                    format!(
                        "Tuples can only store 2 values but got ({})",
                        exprs
                            .iter()
                            .map(|e| e.to_lisp())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                );
            }
            let mut elem_types = Vec::new();
            for arg in args {
                match infer_expr(arg, ctx) {
                    Ok(elem_type) => {
                        elem_types.push(elem_type);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            return Ok(Type::Tuple(elem_types));
        }
    }
    let func_expr = &exprs[0];
    let args = &exprs[1..];

    let mut func_type = infer_expr(func_expr, ctx)?;
    // TODO: remove repetitive logic for 0 args and +=1 args
    if args.is_empty() {
        match func_type {
            Type::Function(_, ret_ty) => {
                return Ok(*ret_ty);
            }
            Type::Var(tv) => {
                let ret_ty = ctx.fresh_var();
                // represent zero-arg function as Function(Box::new(UnitType), Box::new(ret_ty))
                let unit = Type::Unit; // represent by ()
                let func_ty = Type::Function(Box::new(unit), Box::new(ret_ty.clone()));
                ctx.add_constraint(
                    Type::Var(tv.clone()),
                    func_ty,
                    ctx.type_error(TypeErrorVariant::Source, exprs.to_vec())
                );
                return Ok(ret_ty);
            }
            _ => {
                return Err(
                    format!(
                        "Cannot apply non-function type: {}\n{}",
                        func_type,
                        format!(
                            "({})",
                            exprs
                                .into_iter()
                                .map(|e| e.to_lisp())
                                .collect::<Vec<String>>()
                                .join(" ")
                        )
                    )
                );
            }
        }
    }
    for arg in args {
        let arg_type = infer_call_arg(arg, ctx)?;
        match func_type {
            Type::Function(param_ty, ret_ty) => {
                ctx.add_constraint(
                    *param_ty.clone(),
                    arg_type,
                    ctx.type_error(TypeErrorVariant::Call, exprs.to_vec())
                );
                func_type = *ret_ty;
            }
            Type::Var(tv) => {
                // If it's a type variable, assume it's a function type
                {
                    let ret_ty = ctx.fresh_var();
                    let func_ty = Type::Function(
                        Box::new(arg_type.clone()),
                        Box::new(ret_ty.clone())
                    );
                    // Constrain tv = (arg -> ret)
                    ctx.add_constraint(
                        Type::Var(tv.clone()),
                        func_ty,
                        ctx.type_error(TypeErrorVariant::Source, vec![arg.clone()])
                    );
                    func_type = ret_ty;
                }
            }
            _ => {
                return Err(
                    format!(
                        "Cannot apply non-function type: {}\n{}",
                        func_type,
                        format!(
                            "({})",
                            exprs
                                .into_iter()
                                .map(|e| e.to_lisp())
                                .collect::<Vec<String>>()
                                .join(" ")
                        )
                    )
                );
            }
        }
    }
    // Handle calling () -> T
    if args.is_empty() {
        if let Type::Function(param_ty, ret_ty) = &func_type {
            if matches!(**param_ty, Type::Unit) {
                return Ok((**ret_ty).clone());
            }
        }
    }

    Ok(func_type)
}

fn infer_call_arg(arg: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    let inferred = infer_expr(arg, ctx);

    if inferred.is_err() && ctx.last_error_scope.is_none() {
        ctx.last_error_scope = ctx.current_error_scope();
    }

    if ctx.collect_expr_types {
        if let Ok(typ) = &inferred {
            ctx.expr_types.insert(expression_id(arg), typ.clone());
        }
    }

    inferred
}

fn infer_with_builtins_typed_internal(
    expr: &Expression,
    (env, init_id): (TypeEnv, u64),
    user_form_count_for_scope: Option<usize>
) -> Result<(Type, TypedExpression), InferErrorInfo> {
    let mut ctx = InferenceContext {
        env,
        mut_scopes: vec![HashSet::new()],
        lambda_scope_bases: Vec::new(),
        constraints: Vec::new(),
        fresh_var_counter: init_id,
        expr_types: HashMap::new(),
        collect_expr_types: true,
        root_expr_id: expression_id(expr),
        user_form_count_for_scope,
        current_user_top_form: None,
        scope_lambda_path: Vec::new(),
        scope_lambda_counters: vec![0],
        last_error_scope: None,
    };

    let inferred = match infer_expr(expr, &mut ctx) {
        Ok(value) => value,
        Err(message) => {
            let mut partial_typed = build_typed_expression(expr, &ctx.expr_types);
            annotate_effects(&mut partial_typed);
            let partial_typed_ast = Some(partial_typed);
            return Err(InferErrorInfo {
                message,
                scope: ctx.last_error_scope.clone(),
                partial_typed_ast,
            });
        }
    };

    let constraints_vec: Vec<(Type, Type, TypeError)> = ctx.constraints
        .iter()
        .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
        .collect();

    let subst_map = match solve_constraints_list(&constraints_vec) {
        Ok(subst) => subst,
        Err(e) => {
            let mut partial_typed = build_typed_expression(expr, &ctx.expr_types);
            annotate_effects(&mut partial_typed);
            let partial_typed_ast = Some(partial_typed);
            return Err(InferErrorInfo {
                message: e.message,
                scope: e.scope,
                partial_typed_ast,
            });
        }
    };
    let solved_type = apply_subst_map_to_type(&subst_map, &inferred);
    ctx.env.apply_substitution_map(&subst_map);

    let solved_expr_types: HashMap<usize, Type> = ctx.expr_types
        .iter()
        .map(|(id, typ)| (*id, apply_subst_map_to_type(&subst_map, typ)))
        .collect();

    let mut typed_expr = build_typed_expression(expr, &solved_expr_types);
    annotate_effects(&mut typed_expr);
    if let Err(message) = validate_impure_function_name_suffix(&typed_expr) {
        return Err(InferErrorInfo {
            message,
            scope: None,
            partial_typed_ast: Some(typed_expr),
        });
    }
    Ok((solved_type, typed_expr))
}

pub fn infer_with_builtins_typed(
    expr: &Expression,
    (env, init_id): (TypeEnv, u64)
) -> Result<(Type, TypedExpression), String> {
    infer_with_builtins_typed_internal(expr, (env, init_id), None).map_err(|e| e.message)
}

pub fn infer_with_builtins_typed_lsp(
    expr: &Expression,
    (env, init_id): (TypeEnv, u64),
    user_form_count: usize
) -> Result<(Type, TypedExpression), InferErrorInfo> {
    infer_with_builtins_typed_internal(expr, (env, init_id), Some(user_form_count))
}
