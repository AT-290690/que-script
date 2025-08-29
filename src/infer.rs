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
}

// Main type inference function
pub fn infer_type(expr: &Expression, env: &TypeEnv) -> Result<Type, String> {
    let mut ctx = InferenceContext::with_env(env.clone());
    let typ = infer_expr(expr, &mut ctx)?;

    // // Solve constraints
    // let mut subst = Substitution::empty();
    // for (t1, t2) in &ctx.constraints {
    //     let unifier = crate::types::unify(t1, t2)?;
    //     subst = subst.compose(&unifier);
    // }
    // Ok(subst.apply(&typ))
    let subst = solve_constraints(ctx.constraints)?;
    Ok(subst.apply(&typ))
}

// Type inference for expressions
fn infer_expr(expr: &Expression, ctx: &mut InferenceContext) -> Result<Type, String> {
    match expr {
        Expression::Atom(_) => Ok(Type::Int),

        Expression::Word(name) => {
            if let Some(scheme) = ctx.env.get(name) {
                Ok(crate::types::instantiate(&scheme))
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
    if args.len() < 1 {
        return Err("Lambda requires at least a body".to_string());
    }

    let param_count = args.len() - 1;
    let body = &args[param_count];

    // Extract parameter names
    let mut param_names = Vec::new();
    for i in 0..param_count {
        if let Expression::Word(name) = &args[i] {
            param_names.push(name.clone());
        } else {
            return Err("Lambda parameters must be variable names".to_string());
        }
    }

    // Create fresh type variables for parameters
    let mut param_types = Vec::new();
    for _ in 0..param_count {
        param_types.push(ctx.fresh_var());
    }

    // Extend environment with parameter types
    let mut new_env = ctx.env.clone();
    for (name, typ) in param_names.iter().zip(param_types.iter()) {
        new_env.insert(name.clone(), TypeScheme::monotype(typ.clone()));
    }

    // Infer body type
    let mut body_ctx = InferenceContext::with_env(new_env);
    let body_type = infer_expr(body, &mut body_ctx)?;

    // Build function type
    let mut func_type = body_type;
    for param_type in param_types.iter().rev() {
        func_type = Type::Function(Box::new(param_type.clone()), Box::new(func_type));
    }

    // Add constraints from body context
    ctx.constraints.extend(body_ctx.constraints);

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
        let value_type = infer_expr(value_expr, ctx)?;

        // 🔑 apply current substitutions from constraints
        let mut subst = Substitution::empty();
        for (t1, t2) in &ctx.constraints {
            let s = unify(t1, t2)?;
            subst = subst.compose(&s);
        }
        let solved_type = subst.apply(&value_type);

        // now generalize the solved type
        let scheme = generalize(&ctx.env, solved_type);
        ctx.env.insert(var_name.clone(), scheme);
        Ok(Type::Var(TypeVar::new(0))) // Return a fresh type variable
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
                return Ok(Type::List(Box::new(ctx.fresh_var())));
            }
            let mut elem_types = Vec::new();
            for arg in args {
                elem_types.push(infer_expr(arg, ctx)?);
            }
            let first = elem_types[0].clone();
            for t in &elem_types[1..] {
                ctx.add_constraint(first.clone(), t.clone());
            }
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
            _ => return Err(format!("Cannot apply non-function type: {}", func_type)),
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
            vec![TypeVar::new(0)],
            Type::Function(
                Box::new(Type::List(Box::new(Type::Var(TypeVar::new(0))))),
                Box::new(Type::Int),
            ),
        ),
    );

    env.insert(
        "get".to_string(),
        TypeScheme::new(
            vec![TypeVar::new(0)],
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
            vec![TypeVar::new(0)],
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

    // Loop function
    env.insert(
        "loop".to_string(),
        TypeScheme::new(
            vec![TypeVar::new(0)],
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
            vec![TypeVar::new(0)], // T0
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
    env.insert(
        "boolean".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
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
