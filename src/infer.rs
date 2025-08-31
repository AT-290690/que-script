use crate::lisp::Expression;
use crate::types::{
    generalize, solve_constraints, unify, Substitution, Type, TypeEnv, TypeScheme, TypeVar,
};
use std::collections::HashMap;

// Type inference context
pub struct InferenceContext {
    pub env: TypeEnv,
    pub constraints: Vec<(Type, Type)>,
    pub fresh_var_counter: u64,
}

impl InferenceContext {
    pub fn new() -> Self {
        InferenceContext {
            env: TypeEnv::new(),
            constraints: Vec::new(),
            fresh_var_counter: 0,
        }
    }

    pub fn with_env(env: TypeEnv) -> Self {
        InferenceContext {
            env,
            constraints: Vec::new(),
            fresh_var_counter: 0,
        }
    }

    pub fn add_constraint(&mut self, t1: Type, t2: Type) {
        self.constraints.push((t1, t2));
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

fn find_occurs_issues(constraints: &Vec<(Type, Type)>) {
    use std::collections::HashSet;

    eprintln!(
        "Examining {} constraints for occurs-check issues...",
        constraints.len()
    );
    for (i, (t1, t2)) in constraints.iter().enumerate() {
        // gather free vars on both sides (you already have free_vars implementations)
        let free1 = t1.free_vars();
        let free2 = t2.free_vars();

        // any var id that appears in lhs and in inner structure of rhs?
        for id in free1.iter() {
            // If rhs contains the same id -> cycle candidate
            if free2.contains(id) {
                eprintln!("POTENTIAL CYCLE constraint #{}: {}  ~  {}", i, t1, t2);
            }
        }

        // Also check symmetric
        for id in free2.iter() {
            if t1.free_vars().contains(id) {
                eprintln!(
                    "POTENTIAL CYCLE (other side) constraint #{}: {}  ~  {}",
                    i, t1, t2
                );
            }
        }
    }
}

// Type inference for expressions
fn infer_expr(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    match expr {
        Expression::Atom(_) => Ok(Type::Int),

        Expression::Word(name) => {
            if let Some(scheme) = ctx.env.get(name) {
                Ok(ctx.instantiate(&scheme))
            } else {
                Err(format!("Undefined variable: {}", name))
            }
        }

        Expression::Apply(exprs) => {
            if exprs.is_empty() {
                return Err("Empty application".to_string());
            }

            if let Expression::Word(func_name) = &exprs[0] {
                match func_name.as_str() {
                    "lambda" => infer_lambda(&exprs[1..], ctx),
                    "if" => infer_if(&exprs[1..], ctx),
                    "let" => infer_let(&exprs[1..], ctx),
                    "do" => infer_do(&exprs[1..], ctx),
                    _ => infer_function_call(exprs, ctx),
                }
            } else {
                infer_function_call(exprs, ctx)
            }
        }
    }
}

// Type inference for lambda expressions
fn infer_lambda(args: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if args.is_empty() {
        return Err("Lambda requires at least a body".to_string());
    }

    let param_count = args.len() - 1;
    let body = &args[param_count];

    let mut param_names = Vec::new();
    for i in 0..param_count {
        if let Expression::Word(name) = &args[i] {
            param_names.push(name.clone());
        } else {
            return Err("Lambda parameters must be variable names".to_string());
        }
    }

    // fresh type vars for params
    let mut param_types = Vec::new();
    for _ in 0..param_count {
        param_types.push(ctx.fresh_var());
    }

    // extend env in-place (by swap)
    let saved_env = ctx.env.clone();
    let mut extended_env = saved_env.clone();
    for (name, ty) in param_names.iter().zip(param_types.iter()) {
        extended_env.insert(name.clone(), TypeScheme::monotype(ty.clone()));
    }
    let old_env = std::mem::replace(&mut ctx.env, extended_env);

    // infer body in SAME ctx (no fresh ctx)
    let body_type = infer_expr(body, ctx)?;

    // restore env
    ctx.env = old_env;

    // build curried function type (params right-to-left)
    let mut func_type = body_type;
    for p in param_types.iter().rev() {
        func_type = Type::Function(Box::new(p.clone()), Box::new(func_type));
    }
    Ok(func_type)
}

// Type inference for if expressions
fn infer_if(args: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if args.len() != 3 {
        return Err("If requires exactly 3 arguments: condition, then, else".to_string());
    }

    let condition = &args[0];
    let then_expr = &args[1];
    let else_expr = &args[2];

    // Infer condition type - should be Bool
    let cond_type = infer_expr(condition, ctx)?;
    ctx.add_constraint(cond_type, Type::Bool);

    // Infer then and else types
    let then_type = infer_expr(then_expr, ctx)?;
    let else_type = infer_expr(else_expr, ctx)?;

    // Both branches must have the same type
    ctx.add_constraint(then_type.clone(), else_type);

    Ok(then_type)
}

// Type inference for let expressions
fn infer_let(args: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if args.len() != 2 {
        return Err("Let requires exactly 2 arguments: variable and value".to_string());
    }

    let var_expr = &args[0];
    let value_expr = &args[1];

    if let Expression::Word(var_name) = var_expr {
        let value_ty = infer_expr(value_expr, ctx)?;

        // solve current constraints → compose a single Substitution S
        let mut subst = Substitution::empty();
        for (t1, t2) in &ctx.constraints {
            let s = unify(t1, t2)?;
            subst = subst.compose(&s);
        }

        // apply S to the value AND to the env
        let solved_value_ty = subst.apply(&value_ty);
        let solved_env = ctx.env.substitute(&subst.map);

        // generalize against the solved env (so lambda params remain monomorphic)
        let scheme = generalize(&solved_env, solved_value_ty);

        // (optional but helpful) keep ctx.env solved too
        ctx.env = solved_env;
        ctx.env.insert(var_name.clone(), scheme);

        // your language: (let ...) returns 0/Int sentinel
        Ok(Type::Int)
    } else {
        Err("Let variable must be a variable name".to_string())
    }
}

// Type inference for do expressions
fn infer_do(args: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if args.is_empty() {
        return Err("Do requires at least one expression".to_string());
    }

    let mut last_type = Type::Int; // Default type

    for expr in args {
        last_type = infer_expr(expr, ctx)?;
    }

    Ok(last_type)
}
//
// Type inference for function calls
fn infer_function_call(exprs: &[Expression], ctx: &mut InferenceContext) -> Result<Type, String> {
    if exprs.is_empty() {
        return Err("Function call requires at least a function".to_string());
    }

    // Special handling for array before anything else
    if let Expression::Word(name) = &exprs[0] {
        if name == "array" {
            let args = &exprs[1..];
            if args.is_empty() {
                return Ok(Type::List(Box::new(ctx.fresh_var()))); // Empty array case
            }

            let mut elem_types = Vec::new();
            for arg in args {
                let elem_type = infer_expr(arg, ctx)?;
                elem_types.push(elem_type);
            }

            let first = elem_types[0].clone();
            for t in &elem_types[1..] {
                ctx.add_constraint(first.clone(), t.clone()); // Enforce all elements have the same type
            }

            // Return the type of the array (List of the first element type)
            return Ok(Type::List(Box::new(first)));
        }
    }
    let func_expr = &exprs[0];
    let args = &exprs[1..];

    let mut func_type = infer_expr(func_expr, ctx)?;
    for arg in args {
        match func_type {
            Type::Function(param_ty, ret_ty) => {
                let arg_ty = infer_expr(arg, ctx)?;
                ctx.add_constraint(*param_ty.clone(), arg_ty);
                func_type = *ret_ty;
            }
            Type::Var(tv) => {
                // If it's a type variable, assume it's a function type
                let arg_ty = infer_expr(arg, ctx)?;
                let ret_ty = ctx.fresh_var();
                let func_ty = Type::Function(Box::new(arg_ty.clone()), Box::new(ret_ty.clone()));
                // Constrain tv = (arg -> ret)
                ctx.add_constraint(Type::Var(tv.clone()), func_ty);

                func_type = ret_ty;
            }
            _ => {
                return Err(format!("Cannot apply non-function type: {}", func_type));
            }
        }
    }

    Ok(func_type)
}

// Built-in function type signatures
pub fn create_builtin_environment() -> TypeEnv {
    let mut env = TypeEnv::new();

    // Arithmetic operations
    env.insert(
        "+".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    env.insert(
        "-".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    env.insert(
        "*".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    env.insert(
        "/".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    env.insert(
        "mod".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    // Comparison operations
    env.insert(
        "=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    env.insert(
        "<".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    env.insert(
        ">".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    env.insert(
        "<=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    env.insert(
        ">=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    // Logical operations
    env.insert(
        "and".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    env.insert(
        "or".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    env.insert(
        "not".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
    );

    // Array operations
    env.insert(
        "length".to_string(),
        TypeScheme::new(
            vec![0],
            Type::Function(
                Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
                Box::new(Type::Int),
            ),
        ),
    );

    env.insert(
        "get".to_string(),
        TypeScheme::new(
            vec![0],
            Type::Function(
                Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
                Box::new(Type::Function(
                    Box::new(Type::Int),
                    Box::new(Type::Var(TypeVar::new(0))),
                )),
            ),
        ),
    );

    env.insert(
        "set!".to_string(),
        TypeScheme::new(
            vec![0],
            Type::Function(
                Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
                Box::new(Type::Function(
                    Box::new(Type::Int),
                    Box::new(Type::Function(
                        Box::new(Type::Var(TypeVar::new(0))),
                        Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
                    )),
                )),
            ),
        ),
    );

    env.insert(
        "pop!".to_string(),
        TypeScheme::new(
            vec![0],
            Type::Function(
                Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
                Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
            ),
        ),
    );

    // Loop function
    env.insert(
        "loop".to_string(),
        TypeScheme::new(
            vec![0],
            Type::Function(
                Box::new(Type::Bool),
                Box::new(Type::Function(
                    Box::new(Type::Var(TypeVar::new(0))),
                    Box::new(Type::Int),
                )),
            ),
        ),
    );

    // dotimes : Int -> Int -> (Int -> T0) -> Int
    env.insert(
        "dotimes".to_string(),
        TypeScheme::new(
            vec![0], // T0
            Type::Function(
                Box::new(Type::Int), // start
                Box::new(Type::Function(
                    Box::new(Type::Int), // count
                    Box::new(Type::Function(
                        Box::new(Type::Function(
                            Box::new(Type::Int),
                            Box::new(Type::Var(TypeVar::new(0))),
                        )), // body
                        Box::new(Type::Int), // return type
                    )),
                )),
            ),
        ),
    );

    env
}

// Helper function to infer type with built-in environment
pub fn infer_with_builtins(expr: &Expression) -> Result<Type, String> {
    let mut ctx = InferenceContext {
        env: create_builtin_environment(),
        constraints: Vec::new(),
        fresh_var_counter: 0,
    };

    let typ = infer_expr(expr, &mut ctx)?;

    // println!("Constraints:");
    // for (t1, t2) in &ctx.constraints {
    //     println!("  {}  ~  {}", t1, t2);
    // }
    // Solve constraints
    let mut subst = Substitution::empty();
    for (t1, t2) in &ctx.constraints {
        let s = crate::types::unify(t1, t2)?;
        subst = subst.compose(&s);
    }
    // let subst = solve_constraints(ctx.constraints)?;
    Ok(subst.apply(&typ))
}
