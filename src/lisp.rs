use core::panic;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Expression {
    Atom(i32),
    Word(String),
    Apply(Vec<Expression>),
}
#[derive(Clone)]
pub enum Evaluated {
    Function(Rc<dyn Fn(Vec<Expression>, Rc<RefCell<Env>>, Rc<RefCell<Env>>) -> Evaluated>),
    Number(i32),
    Vector(Rc<RefCell<Vec<Evaluated>>>),
}

impl fmt::Debug for Evaluated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Evaluated::Number(value) => write!(f, "{}", value),
            Evaluated::Function(_) => write!(f, "Function"),
            Evaluated::Vector(arr) => {
                let arr_ref = arr.borrow();
                let elements: Vec<String> = arr_ref.iter().map(|x| format!("{:?}", x)).collect();
                write!(f, "[{}]", elements.join(" "))
            }
        }
    }
}

impl Expression {
    pub fn to_rust(&self) -> String {
        match self {
            Expression::Atom(n) => format!("Expression::Atom({})", n),
            Expression::Word(w) => format!("Expression::Word({:?}.to_string())", w),
            Expression::Apply(exprs) => {
                let inner: Vec<String> = exprs.iter().map(|e| e.to_rust()).collect();
                format!("Expression::Apply(vec![{}])", inner.join(", "))
            }
        }
    }
}

#[derive(Clone)]
pub struct Env {
    vars: HashMap<String, Evaluated>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn new() -> Self {
        Env {
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Rc<RefCell<Env>>) -> Self {
        Env {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    fn get(&self, name: &str) -> Option<Evaluated> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: Evaluated) {
        self.vars.insert(name, value);
    }
}

fn evaluate(exp: &Expression, env: Rc<RefCell<Env>>, defs: Rc<RefCell<Env>>) -> Evaluated {
    match exp {
        Expression::Atom(value) => Evaluated::Number(*value),
        Expression::Word(name) => {
            let env_ref = env.borrow();
            if let Some(var) = env_ref.get(name) {
                return var;
            }
            let defs_ref = defs.borrow();
            if let Some(var) = defs_ref.get(name) {
                return var;
            }
            panic!("Undefined variable: {}", name)
        }
        Expression::Apply(exprs) => {
            if let Expression::Word(name) = &exprs[0] {
                let env_ref = env.borrow();
                if let Some(var) = env_ref.get(name) {
                    match var {
                        Evaluated::Function(func) => {
                            return func(exprs[1..].to_vec(), Rc::clone(&env), Rc::clone(&defs))
                        }
                        _ => panic!("Cannot apply a non-lambda value"),
                    }
                }
                let defs_ref = defs.borrow();
                if let Some(var) = defs_ref.get(name) {
                    match var {
                        Evaluated::Function(func) => {
                            return func(exprs[1..].to_vec(), Rc::clone(&env), Rc::clone(&defs))
                        }
                        _ => panic!("Cannot apply a non-lambda value"),
                    }
                }
                panic!("Function not found: {}", name)
            }
            panic!("Invalid lambda application")
        }
    }
}

fn init() -> Rc<RefCell<Env>> {
    Rc::new(RefCell::new(Env {
        vars: HashMap::from([
            (
                "loop-finish".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let func_val = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match func_val {
                            Evaluated::Function(f) => {
                                while let Evaluated::Number(value) =
                                    evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs))
                                {
                                    if value == 1 {
                                        f(vec![], Rc::clone(&env), Rc::clone(&defs));
                                    } else {
                                        break;
                                    }
                                }
                            }
                            _ => panic!("while: second argument must be a lambda"),
                        }

                        return Evaluated::Number(0);
                    },
                )),
            ),
            (
                "loop".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        if args.len() != 3 {
                            panic!("loop: expects 3 arguments (start, end, func)");
                        }

                        let start_val = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let end_val = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        let func_val = evaluate(&args[2], Rc::clone(&env), Rc::clone(&defs));

                        let start = match start_val {
                            Evaluated::Number(n) => n,
                            _ => panic!("loop: start must be a number"),
                        };
                        let end = match end_val {
                            Evaluated::Number(n) => n,
                            _ => panic!("loop: end must be a number"),
                        };

                        match func_val {
                            Evaluated::Function(f) => {
                                for i in start..end {
                                    f(vec![Expression::Atom(i)], Rc::clone(&env), Rc::clone(&defs));
                                }

                                Evaluated::Number(0)
                            }
                            _ => panic!("loop: third argument must be a lambda"),
                        }
                    },
                )),
            ),
            (
                "array".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let evaluated_args = args
                            .into_iter()
                            .map(|arg| evaluate(&arg, Rc::clone(&env), Rc::clone(&defs)))
                            .collect();
                        Evaluated::Vector(Rc::new(RefCell::new(evaluated_args)))
                    },
                )),
            ),
            (
                "length".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                            Evaluated::Vector(arr) => Evaluated::Number(arr.borrow().len() as i32),
                            _ => panic!("First argument must be an array"),
                        }
                    },
                )),
            ),
            (
                "get".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                            Evaluated::Vector(arr) => {
                                let index = evaluate(&args[1], env, defs);
                                match index {
                                    Evaluated::Number(index) => {
                                        let len: usize = arr.borrow().len();
                                        if index >= 0 && index < (len as i32) {
                                            arr.borrow_mut().get(index as usize).unwrap().clone()
                                        } else {
                                            panic!("Index is outside ofthe array bounds")
                                        }
                                    }
                                    _ => panic!("Second argument of get must be a number"),
                                }
                            }
                            _ => panic!("First argument must be an array"),
                        }
                    },
                )),
            ),
            (
                "set!".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                            Evaluated::Vector(arr) => {
                                let index = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                                match index {
                                    Evaluated::Number(index) => {
                                        {
                                            let len: usize = arr.borrow().len();
                                            if index >= 0 && index < (len as i32) {
                                                arr.borrow_mut()[index as usize] = evaluate(
                                                    &args[2],
                                                    Rc::clone(&env),
                                                    Rc::clone(&defs),
                                                );
                                            } else if index == (len as i32) {
                                                arr.borrow_mut().push(evaluate(
                                                    &args[2],
                                                    Rc::clone(&env),
                                                    Rc::clone(&defs),
                                                ));
                                            } else {
                                                panic!("Index is outside ofthe array bounds");
                                            }
                                        }
                                        Evaluated::Number(0)
                                    }
                                    _ => panic!("Second argument of get must be a number"),
                                }
                            }
                            _ => panic!("First argument must be an array"),
                        }
                    },
                )),
            ),
            (
                "pop!".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                            Evaluated::Vector(arr) => {
                                arr.borrow_mut().pop();
                                Evaluated::Number(0)
                            }
                            _ => panic!("First argument must be an array"),
                        }
                    },
                )),
            ),
            (
                "+".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a + b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "-".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a - b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "*".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a * b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "/".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a / b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "mod".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a % b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "&".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a & b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "|".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a | b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "^".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a ^ b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "~".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        match a {
                            Evaluated::Number(a) => Evaluated::Number(!a),
                            _ => panic!("Argument must be be number"),
                        }
                    },
                )),
            ),
            (
                ">>".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a >> b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "<<".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(a << b)
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "if".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let condition = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        match condition {
                            Evaluated::Number(condition) => {
                                if condition == 1 {
                                    evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs))
                                } else {
                                    evaluate(&args[2], Rc::clone(&env), Rc::clone(&defs))
                                }
                            }
                            _ => panic!("First argument must be a 1 or 0"),
                        }
                    },
                )),
            ),
            (
                ">".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(if a > b { 1 } else { 0 })
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "<".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(if a < b { 1 } else { 0 })
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                ">=".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(if a >= b { 1 } else { 0 })
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "<=".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(if a <= b { 1 } else { 0 })
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "=".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        match (a, b) {
                            (Evaluated::Number(a), Evaluated::Number(b)) => {
                                Evaluated::Number(if a == b { 1 } else { 0 })
                            }
                            _ => panic!("Both arguments must be numbers"),
                        }
                    },
                )),
            ),
            (
                "not".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let condition: Evaluated =
                            evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        match condition {
                            Evaluated::Number(condition) => {
                                if condition != 0 {
                                    Evaluated::Number(0)
                                } else {
                                    Evaluated::Number(1)
                                }
                            }
                            _ => panic!("Argument must be a 1 or 0"),
                        }
                    },
                )),
            ),
            (
                "and".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                            Evaluated::Number(a) => {
                                if a == 1 {
                                    match evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs)) {
                                        Evaluated::Number(b) => {
                                            if b == 1 {
                                                Evaluated::Number(1)
                                            } else {
                                                Evaluated::Number(0)
                                            }
                                        }
                                        _ => panic!("First argument must be a 1 or 0"),
                                    }
                                } else {
                                    Evaluated::Number(0)
                                }
                            }
                            _ => panic!("First argument must be a 1 or 0"),
                        }
                    },
                )),
            ),
            (
                "or".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                            Evaluated::Number(a) => {
                                if a == 0 {
                                    match evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs)) {
                                        Evaluated::Number(b) => {
                                            if b == 1 {
                                                Evaluated::Number(1)
                                            } else {
                                                Evaluated::Number(0)
                                            }
                                        }
                                        _ => panic!("First argument must be a 1 or 0"),
                                    }
                                } else {
                                    Evaluated::Number(1)
                                }
                            }
                            _ => panic!("First argument must be a 1 or 0"),
                        }
                    },
                )),
            ),
            (
                "do".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        let mut last_result: Evaluated = Evaluated::Number(0);
                        for expr in args {
                            last_result = evaluate(&expr, Rc::clone(&env), Rc::clone(&defs));
                        }
                        last_result
                    },
                )),
            ),
            (
                "lambda".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     scope: Rc<RefCell<Env>>|
                     -> Evaluated {
                        if args.len() < 1 {
                            panic!("lambda expects at least one argument: the body");
                        }
                        let params: Vec<String> = args[0..args.len() - 1]
                            .iter()
                            .filter_map(|param| match param {
                                Expression::Word(name) => Some(name.clone()),
                                _ => None,
                            })
                            .collect();
                        let body: Expression = args[args.len() - 1].clone();
                        Evaluated::Function(Rc::new(
                            move |lambda_args: Vec<Expression>,
                                  _env: Rc<RefCell<Env>>,
                                  defs: Rc<RefCell<Env>>| {
                                if lambda_args.len() != params.len() {
                                    panic!(
                                        "Expected {} arguments, but got {}",
                                        params.len(),
                                        lambda_args.len()
                                    );
                                }
                                let local_defs =
                                    Rc::new(RefCell::new(Env::with_parent(Rc::clone(&scope))));
                                {
                                    let mut local_defs_ref = local_defs.borrow_mut();
                                    for (param, arg) in params.iter().zip(lambda_args.iter()) {
                                        let value =
                                            evaluate(arg, Rc::clone(&env), Rc::clone(&defs));
                                        local_defs_ref.set(param.clone(), value);
                                    }
                                }
                                evaluate(&body, Rc::clone(&env), Rc::clone(&local_defs))
                            },
                        ))
                    },
                )),
            ),
            (
                "let".to_string(),
                Evaluated::Function(Rc::new(
                    |args: Vec<Expression>,
                     env: Rc<RefCell<Env>>,
                     defs: Rc<RefCell<Env>>|
                     -> Evaluated {
                        if args.len() != 2 {
                            panic!(
                                "let expects exactly two arguments: a variable name and a value"
                            );
                        }
                        if let Expression::Word(var_name) = &args[0] {
                            let value = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                            defs.borrow_mut().set(var_name.clone(), value);
                            return Evaluated::Number(0);
                        } else {
                            panic!("First argument to 'let' must be a variable name")
                        }
                    },
                )),
            ),
        ]),
        parent: None,
    }))
}
pub fn run(expr: &Expression) -> Evaluated {
    let env = init();
    let defs: Rc<RefCell<Env>> = Rc::new(RefCell::new(Env::new()));
    return evaluate(&expr, Rc::clone(&env), Rc::clone(&defs));
}
