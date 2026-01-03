use crate::parser::Expression;
use crate::types::{generalize, Type, TypeEnv, TypeScheme, TypeVar};
use std::collections::{HashMap, VecDeque};

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
}

fn src_to_pretty(src: &TypeError) -> String {
    let joined = src
        .expr
        .iter()
        .map(|e| e.to_lisp())
        .collect::<Vec<_>>()
        .join(" ");
    match src.variant {
        TypeErrorVariant::Vector => format!("Error! (vector {})", joined),
        TypeErrorVariant::Call => format!("Error! ({})", joined),
        TypeErrorVariant::IfCond => format!("Error! Condition must be Bool\n(if {})", joined),
        TypeErrorVariant::IfBody => {
            format!(
                "Error! Concequent and alternative must match types\n(if {})",
                joined
            )
        }
        TypeErrorVariant::Source => joined,
    }
}
pub struct InferenceContext {
    pub env: TypeEnv,
    pub constraints: Vec<(Type, Type, TypeError)>,
    pub fresh_var_counter: u64,
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
}

fn infer_expr(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    match expr {
        Expression::Int(_) => Ok(Type::Int),
        Expression::Float(_) => Ok(Type::Float),

        Expression::Word(name) => {
            if let Some(scheme) = ctx.env.get(name) {
                Ok(ctx.instantiate(&scheme))
            } else {
                Err(format!("Error! Undefined variable: {}", name))
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
                    "do" => infer_do(&exprs, ctx),
                    _ => infer_function_call(exprs, ctx),
                }
            } else {
                infer_function_call(exprs, ctx)
            }
        }
    }
}

fn parse_type_hint(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    match expr {
        Expression::Word(name) => match name.as_str() {
            "Int" => Ok(Type::Int),
            "Float" => Ok(Type::Float),
            "Bool" => Ok(Type::Bool),
            "Char" => Ok(Type::Char),
            _ => Ok(ctx.fresh_var()), // unknown type name
        },

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
                        return Err(format!(
                            "Error! Tuple type must have at least one element: {}",
                            expr.to_lisp()
                        ));
                    }
                    let mut elems = Vec::new();
                    for elem_expr in &items[1..] {
                        elems.push(parse_type_hint(elem_expr, ctx)?);
                    }
                    return Ok(Type::Tuple(elems));
                }
            }
            Err(format!(
                "Error! Invalid type hint syntax: {}",
                expr.to_lisp()
            ))
        }

        _ => Err(format!("Error! Invalid type hint: {}", expr.to_lisp())),
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
        return Err("Error! as expects exactly two arguments: (as expr Type)".to_string());
    }

    // Infer both sides
    let expr_type = infer_expr(&args[0], ctx)?;
    let type_hint = parse_type_hint(&args[1], ctx)?;

    // Handle tuple special case directly — before arity logic
    match (&expr_type, &type_hint) {
        (Type::Tuple(expr_elems), Type::Tuple(hint_elems)) => {
            if expr_elems.len() != hint_elems.len() {
                return Err(format!(
                    "Error! Tuple length mismatch in as: {} vs {}\n(as {})",
                    expr_elems.len(),
                    hint_elems.len(),
                    args.iter()
                        .map(|e| e.to_lisp())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }

            // Create constraints for each element
            for (e, h) in expr_elems.iter().zip(hint_elems.iter()) {
                ctx.add_constraint(
                    e.clone(),
                    h.clone(),
                    TypeError {
                        variant: TypeErrorVariant::Source,
                        expr: args.to_vec(),
                    },
                );
            }

            return Ok(type_hint);
        }
        (Type::Tuple(_), _) | (_, Type::Tuple(_)) => {
            return Err(format!(
                "Error! Cannot cast between tuple and non-tuple types\n(as {})",
                args.iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
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
        return Err(format!(
            "Error! Type variable in as cannot represent deeper nesting: {} vs {}",
            expr_type, type_hint
        ));
    }

    // Check arity mismatch for lists/functions
    if !is_expr_var && expr_arity != hint_arity {
        return Err(format!(
            "Error! Type arity mismatch in as: left has arity {}, right has arity {} ({} vs {})\n(as {})",
            expr_arity,
            hint_arity,
            expr_type,
            type_hint,
            args.iter().map(|e| e.to_lisp()).collect::<Vec<_>>().join(" ")
        ));
    }

    // Array element restriction)
    if expr_arity > 0 {
        let inner_expr = inner_type(&expr_type);
        let inner_hint = inner_type(&type_hint);

        match (inner_expr, inner_hint) {
            (Type::Int, Type::Int)
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
                return Err(format!(
                    "Error! Invalid array cast in as: cannot cast {} to {}\n(as {})",
                    expr_type,
                    type_hint,
                    args.iter()
                        .map(|e| e.to_lisp())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
    }

    Ok(type_hint)
}

// Type inference for lambda expressions
fn infer_lambda(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.is_empty() {
        return Err("Error! Lambda requires a body".to_string());
    }

    let param_count = args.len() - 1;
    let body = &args[param_count];

    // Extract parameter names
    let mut param_names = Vec::new();
    for i in 0..param_count {
        if let Expression::Word(name) = &args[i] {
            param_names.push(name.clone());
        } else {
            return Err(format!(
                "Error! Lambda parameters must be variable names\n({})",
                exprs
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
        }
    }

    // Create fresh type vars
    let mut param_types = Vec::new();
    for _ in 0..param_count {
        param_types.push(ctx.fresh_var());
    }

    // Enter new lexical scope
    ctx.env.enter_scope();

    // Insert parameters
    for (name, typ) in param_names.iter().zip(param_types.iter()) {
        ctx.env
            .insert(name.clone(), TypeScheme::monotype(typ.clone()))
            .map_err(|e| format!("Error! in lambda: {}", e))?;
    }

    // Infer body type
    let body_type = infer_expr(body, ctx)?;

    // Exit scope
    ctx.env.exit_scope();

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
        return Err("Error! If requires exactly 3 arguments: condition, then, else".to_string());
    }

    let condition = &args[0];
    let then_expr = &args[1];
    let else_expr = &args[2];

    // Infer condition type - should be Bool
    let cond_type = infer_expr(condition, ctx)?;
    ctx.add_constraint(
        cond_type.clone(),
        Type::Bool,
        TypeError {
            variant: TypeErrorVariant::IfCond,
            expr: args.to_vec(),
        },
    );
    // Infer then and else types
    let then_type = infer_expr(then_expr, ctx)?;
    let else_type = infer_expr(else_expr, ctx)?;

    // Both branches must have the same type
    ctx.add_constraint(
        then_type.clone(),
        else_type,
        TypeError {
            variant: TypeErrorVariant::IfBody,
            expr: args.to_vec(),
        },
    );

    Ok(then_type)
}
fn is_nonexpansive(expr: &Expression) -> bool {
    match expr {
        Expression::Word(_) | Expression::Int(_) | Expression::Float(_) => true,

        Expression::Apply(list) if !list.is_empty() => match &list[0] {
            Expression::Word(name) if name == "lambda" => true,
            // This is commented out because it will otherwise cause a bug with mutaiton (set!) inference
            // and keep the vector polymorphic for empty nested vectors [[]]
            // Expression::Word(name) if name == "vector" => !list[1..].is_empty(),
            _ => false,
        },

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
            Some(ty) => match ty {
                Type::Var(ref v) if v.id != id => {
                    // path compress
                    let rep = self.find_var(v.id);
                    // store the rep
                    self.binds.insert(id, rep.clone());
                    rep
                }
                other => other,
            },
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
            Type::Tuple(items) => Type::Tuple(items.iter().map(|t| self.apply(t)).collect()),
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
            return Err(format!(
                "Occurs check failed: t{} occurs in {:?}",
                var_id, ty
            ));
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
    constraints: &Vec<(Type, Type, TypeError)>,
) -> Result<HashMap<u64, Type>, String> {
    let mut unifier = Unifier::new();
    let mut work: VecDeque<(Type, Type, TypeError)> = VecDeque::new();

    for (a, b, src) in constraints.iter() {
        work.push_back((a.clone(), b.clone(), src.clone()));
    }

    while let Some((left, right, src)) = work.pop_front() {
        let left_ap = unifier.apply(&left);
        let right_ap = unifier.apply(&right);

        match (left_ap.clone(), right_ap.clone()) {
            (Type::Var(v), ty) | (ty, Type::Var(v)) => {
                if let Err(e) = unifier.bind_var(v.id, ty) {
                    // attach source info and return
                    return Err(format!("{}\n{}", src_to_pretty(&src), e));
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
                    return Err(format!(
                        "Error! Cannot unify tuples of different lengths ({} vs {})\n{}",
                        a_items.len(),
                        b_items.len(),
                        src_to_pretty(&src),
                    ));
                }
                for (ai, bi) in a_items.into_iter().zip(b_items.into_iter()) {
                    work.push_back((ai, bi, src.clone()));
                }
            }
            (a2, b2) if a2 == b2 => {} // ok
            (a2, b2) => {
                // can't unify, attach source and return
                return Err(format!(
                    "Error! Cannot unify {} with {}\n{}",
                    a2,
                    b2,
                    src_to_pretty(&src),
                ));
            }
        }
    }

    Ok(unifier.into_substitution())
}

pub fn apply_subst_map_to_type(subst: &HashMap<u64, Type>, ty: &Type) -> Type {
    match ty {
        Type::Var(var) => match subst.get(&var.id) {
            Some(t) => apply_subst_map_to_type(subst, t),
            None => Type::Var(var.clone()),
        },
        Type::List(inner) => Type::List(Box::new(apply_subst_map_to_type(subst, inner))),
        Type::Function(a, b) => Type::Function(
            Box::new(apply_subst_map_to_type(subst, a)),
            Box::new(apply_subst_map_to_type(subst, b)),
        ),
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|it| apply_subst_map_to_type(subst, it))
                .collect(),
        ),
        other => other.clone(),
    }
}
fn infer_rec(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(format!(
            "Error! Let requires exactly 2 arguments: variable and value\n({})",
            exprs
                .iter()
                .map(|e| e.to_lisp())
                .collect::<Vec<_>>()
                .join(" "),
        ));
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    if let Expression::Word(var_name) = var_expr {
        let name = var_name.to_string();

        // assign a fresh monotype placeholder
        let tv = ctx.fresh_var();

        ctx.env
            .insert(name.clone(), TypeScheme::monotype(tv.clone()))?;

        let value_type = infer_expr(value_expr, ctx)?;

        // solve constraints
        let constraints_vec = ctx.constraints.clone();
        let subst_map = solve_constraints_list(&constraints_vec)?;
        let solved_type = apply_subst_map_to_type(&subst_map, &value_type);
        ctx.env.apply_substitution_map(&subst_map);

        // generalize only if nonexpansive
        if is_nonexpansive(value_expr) {
            generalize(&ctx.env, solved_type)
        } else {
            return Err(
                "Error! Only recursive functions allowed for let* optimization".to_string(),
            );
        };

        Ok(Type::Unit)
    } else {
        Err("Error! Let variable must be a variable name".to_string())
    }
}

fn infer_let(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.len() != 2 {
        return Err(format!(
            "Error! Let requires exactly 2 arguments: variable and value\n({})",
            exprs
                .iter()
                .map(|e| e.to_lisp())
                .collect::<Vec<_>>()
                .join(" "),
        ));
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    if let Expression::Word(var_name) = var_expr {
        let value_type = infer_expr(value_expr, ctx)?;

        let constraints_vec: Vec<(Type, Type, TypeError)> = ctx
            .constraints
            .iter()
            .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
            .collect();

        let subst_map = solve_constraints_list(&constraints_vec).map(|e: HashMap<u64, Type>| e)?;

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
        Err("Error! Let variable must be a variable name".to_string())
    }
}

// Type inference for do expressions
fn infer_do(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    let args = &exprs[1..];
    if args.is_empty() {
        return Err("Error! do requires at least one expression".to_string());
    }

    let mut last_type = Type::Unit; // Default type

    for expr in args {
        last_type = infer_expr(expr, ctx)?;
    }

    Ok(last_type)
}
//
// Type inference for function calls
fn infer_function_call(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if exprs.is_empty() {
        return Err("Error! Function call requires at least a function".to_string());
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
                    TypeError {
                        variant: TypeErrorVariant::Vector,
                        expr: args.to_vec(),
                    },
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
                            TypeError {
                                variant: TypeErrorVariant::Vector, // or create Variant::String if desired
                                expr: args.to_vec(),
                            },
                        );
                    }
                    Err(e) => return Err(e),
                }
            }

            return Ok(Type::Char);
        } else if name == "tuple" {
            let args = &exprs[1..];
            if args.len() != 2 {
                return Err(format!(
                    "Error! Tuples can only store 2 values but got {{{}}}",
                    exprs
                        .iter()
                        .map(|e| e.to_lisp())
                        .collect::<Vec<_>>()
                        .join(" "),
                ));
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
                    TypeError {
                        variant: TypeErrorVariant::Source,
                        expr: exprs.to_vec(),
                    },
                );
                return Ok(ret_ty);
            }
            _ => {
                return Err(format!(
                    "Error! Cannot apply non-function type: {}\n{}",
                    func_type,
                    format!(
                        "({})",
                        exprs
                            .into_iter()
                            .map(|e| e.to_lisp())
                            .collect::<Vec<String>>()
                            .join(" ")
                    ),
                ));
            }
        }
    }
    for arg in args {
        match func_type {
            Type::Function(param_ty, ret_ty) => match infer_expr(arg, ctx) {
                Ok(arg_ty) => {
                    ctx.add_constraint(
                        *param_ty.clone(),
                        arg_ty,
                        TypeError {
                            variant: TypeErrorVariant::Call,
                            expr: exprs.to_vec(),
                        },
                    );
                    func_type = *ret_ty;
                }
                Err(e) => {
                    return Err(e);
                }
            },
            Type::Var(tv) => {
                // If it's a type variable, assume it's a function type
                match infer_expr(arg, ctx) {
                    Ok(arg_ty) => {
                        let ret_ty = ctx.fresh_var();
                        let func_ty =
                            Type::Function(Box::new(arg_ty.clone()), Box::new(ret_ty.clone()));
                        // Constrain tv = (arg -> ret)
                        ctx.add_constraint(
                            Type::Var(tv.clone()),
                            func_ty,
                            TypeError {
                                variant: TypeErrorVariant::Source,
                                expr: vec![arg.clone()],
                            },
                        );
                        func_type = ret_ty;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            _ => {
                return Err(format!(
                    "Error! Cannot apply non-function type: {}\n{}",
                    func_type,
                    format!(
                        "({})",
                        exprs
                            .into_iter()
                            .map(|e| e.to_lisp())
                            .collect::<Vec<String>>()
                            .join(" ")
                    ),
                ));
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

// Built-in function type signatures
pub fn create_builtin_environment(mut env: TypeEnv) -> (TypeEnv, u64) {
    // Local fresh-var generator
    let mut fresh_id: u64 = env.scopes.len() as u64;

    let mut fresh_var = || {
        let id = fresh_id;
        fresh_id += 1;
        Type::Var(TypeVar { id })
    };
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "set!".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::List(Box::new(a.clone()))),
                    Box::new(Type::Function(
                        Box::new(Type::Int),
                        Box::new(Type::Function(Box::new(a), Box::new(Type::Unit))),
                    )),
                ),
            ),
        );
    }
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "pop!".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(Box::new(Type::List(Box::new(a))), Box::new(Type::Unit)),
            ),
        );
    }
    {
        let a = fresh_var();
        let _ = env.insert(
            "length".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(Box::new(Type::List(Box::new(a))), Box::new(Type::Int)),
            ),
        );
    }
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "get".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::List(Box::new(a.clone()))),
                    Box::new(Type::Function(Box::new(Type::Int), Box::new(a))),
                ),
            ),
        );
    }
    {
        let a = fresh_var();
        let b = fresh_var();
        let _ = env.insert(
            "fst".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap(), b.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Tuple(vec![a.clone(), b.clone()])),
                    Box::new(a.clone()),
                ),
            ),
        );
    }

    {
        let a = fresh_var();
        let b = fresh_var();
        let _ = env.insert(
            "snd".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap(), b.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Tuple(vec![a.clone(), b.clone()])),
                    Box::new(b.clone()),
                ),
            ),
        );
    }
    {
        let e = fresh_var();

        let _ = env.insert(
            "loop".to_string(),
            TypeScheme::new(
                vec![e.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Int),
                    Box::new(Type::Function(
                        Box::new(Type::Int),
                        Box::new(Type::Function(
                            Box::new(Type::Function(Box::new(Type::Int), Box::new(e.clone()))),
                            Box::new(Type::Unit),
                        )),
                    )),
                ),
            ),
        );
    }
    {
        let e = fresh_var();

        let _ = env.insert(
            "loop-finish".to_string(),
            TypeScheme::new(
                vec![e.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Bool),
                    Box::new(Type::Function(Box::new(e.clone()), Box::new(Type::Unit))),
                ),
            ),
        );
    }
    let _ = env.insert(
        "+".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );
    let _ = env.insert(
        "+.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );
    let _ = env.insert(
        "+#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );

    let _ = env.insert(
        "-".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "-.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    let _ = env.insert(
        "-#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );

    let _ = env.insert(
        "*".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "*.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    let _ = env.insert(
        "*#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );

    let _ = env.insert(
        "/".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "/.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    let _ = env.insert(
        "/#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );
    let _ = env.insert(
        "/.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );
    let _ = env.insert(
        "mod".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );
    let _ = env.insert(
        "mod.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    // Comparison operations
    let _ = env.insert(
        "=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "=.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">=.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<=.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert("true".to_string(), TypeScheme::monotype(Type::Bool));
    let _ = env.insert("false".to_string(), TypeScheme::monotype(Type::Bool));

    let _ = env.insert(
        "=?".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "=#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<=#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">=#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    // Logical operations
    let _ = env.insert(
        "and".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "or".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "not".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
    );

    // Bitwise operations
    let _ = env.insert(
        "&".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "|".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "^".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        ">>".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "<<".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "Int->Float".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Int), Box::new(Type::Float))),
    );

    let _ = env.insert(
        "Float->Int".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Float), Box::new(Type::Int))),
    );

    let _ = env.insert(
        "~".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
    );

    (env, fresh_id) // return env with built-ins and also return next fresh ID
}

pub fn infer_with_builtins(
    expr: &Expression,
    (env, init_id): (TypeEnv, u64),
) -> Result<Type, String> {
    let mut ctx = InferenceContext {
        env,
        constraints: Vec::new(),
        fresh_var_counter: init_id,
    };

    // Infer type — accumulate constraints
    let inferred = infer_expr(expr, &mut ctx)?;

    // Solve constraints once, globally
    let constraints_vec: Vec<(Type, Type, TypeError)> = ctx
        .constraints
        .iter()
        .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
        .collect();

    let subst_map = solve_constraints_list(&constraints_vec).map_err(|e| e.to_string())?;

    // Apply solved substitution map to everything
    let solved_type = apply_subst_map_to_type(&subst_map, &inferred);
    ctx.env.apply_substitution_map(&subst_map);

    Ok(solved_type)
}

pub fn infer_with_builtins_env(
    expr: &Expression,
    (env, init_id): (TypeEnv, u64),
) -> Result<TypeEnv, String> {
    let mut ctx = InferenceContext {
        env,
        constraints: Vec::new(),
        fresh_var_counter: init_id,
    };

    // Infer type — accumulate constraints
    infer_expr(expr, &mut ctx)?;

    // Solve constraints once, globally
    let constraints_vec: Vec<(Type, Type, TypeError)> = ctx
        .constraints
        .iter()
        .map(|(a, b, src)| (a.clone(), b.clone(), src.clone()))
        .collect();

    let subst_map = solve_constraints_list(&constraints_vec).map_err(|e| e.to_string())?;

    // Apply solved substitution map to everything
    // let solved_type = apply_subst_map_to_type(&subst_map, &inferred);
    ctx.env.apply_substitution_map(&subst_map);

    Ok(ctx.env)
}
