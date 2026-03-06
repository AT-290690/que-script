use crate::parser::Expression;
use crate::types::{ generalize, Type, TypeEnv, TypeScheme, TypeVar };
use std::collections::{ HashMap, VecDeque };

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
}

#[derive(Clone, Debug)]
pub struct TypedExpression {
    pub expr: Expression,
    pub typ: Option<Type>,
    pub children: Vec<TypedExpression>,
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
        children,
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
}

fn infer_expr(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    let inferred = match expr {
        Expression::Int(_) => Ok(Type::Int),
        Expression::Float(_) => Ok(Type::Float),

        Expression::Word(name) => {
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
                    "let" => infer_let(&exprs, ctx),
                    "let*" => infer_rec(&exprs, ctx),
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
                "Float" => Ok(Type::Float),
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
            | (Type::Float, Type::Float)
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

    // Enter new lexical scope
    ctx.env.enter_scope();
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
    ctx.env.exit_scope();
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
    ctx.add_constraint(cond_type.clone(), Type::Bool, ctx.type_error(TypeErrorVariant::IfCond, args.to_vec()));
    // Infer then and else types
    let then_type = infer_expr(then_expr, ctx)?;
    let else_type = infer_expr(else_expr, ctx)?;

    // Both branches must have the same type
    ctx.add_constraint(then_type.clone(), else_type, ctx.type_error(TypeErrorVariant::IfBody, args.to_vec()));

    Ok(then_type)
}
fn is_nonexpansive(expr: &Expression) -> bool {
    match expr {
        Expression::Word(_) | Expression::Int(_) | Expression::Float(_) => true,

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
            return Err("Only recursive functions allowed for let* optimization".to_string());
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

// Type inference for do expressions
fn infer_do(expr: &Expression, exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
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
        match func_type {
            Type::Function(param_ty, ret_ty) =>
                match infer_expr(arg, ctx) {
                    Ok(arg_ty) => {
                        ctx.add_constraint(
                            *param_ty.clone(),
                            arg_ty,
                            ctx.type_error(TypeErrorVariant::Call, exprs.to_vec())
                        );
                        func_type = *ret_ty;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            Type::Var(tv) => {
                // If it's a type variable, assume it's a function type
                match infer_expr(arg, ctx) {
                    Ok(arg_ty) => {
                        let ret_ty = ctx.fresh_var();
                        let func_ty = Type::Function(
                            Box::new(arg_ty.clone()),
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
                    Err(e) => {
                        return Err(e);
                    }
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

fn infer_with_builtins_typed_internal(
    expr: &Expression,
    (env, init_id): (TypeEnv, u64),
    user_form_count_for_scope: Option<usize>
) -> Result<(Type, TypedExpression), InferErrorInfo> {
    let mut ctx = InferenceContext {
        env,
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

    let inferred = infer_expr(expr, &mut ctx).map_err(|message| InferErrorInfo {
        message,
        scope: ctx.last_error_scope.clone(),
    })?;

    let constraints_vec: Vec<(Type, Type, TypeError)> = ctx.constraints
        .iter()
        .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
        .collect();

    let subst_map = solve_constraints_list(&constraints_vec).map_err(|e| InferErrorInfo {
        message: e.message,
        scope: e.scope,
    })?;
    let solved_type = apply_subst_map_to_type(&subst_map, &inferred);
    ctx.env.apply_substitution_map(&subst_map);

    let solved_expr_types: HashMap<usize, Type> = ctx.expr_types
        .iter()
        .map(|(id, typ)| (*id, apply_subst_map_to_type(&subst_map, typ)))
        .collect();

    let typed_expr = build_typed_expression(expr, &solved_expr_types);
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
