use crate::lisp::Expression;
use core::panic;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
#[derive(Clone)]
pub enum BiteCodeEvaluated {
    Int(i32),
    Function(Vec<String>, Vec<Instruction>, Rc<RefCell<BiteCodeEnv>>),
    Array(Rc<RefCell<Vec<BiteCodeEvaluated>>>),
}

impl fmt::Debug for BiteCodeEvaluated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BiteCodeEvaluated::Int(value) => write!(f, "{}", value),
            BiteCodeEvaluated::Function(_, _, _) => write!(f, "Function"),
            BiteCodeEvaluated::Array(arr) => {
                let arr_ref = arr.borrow();
                let elements: Vec<String> = arr_ref.iter().map(|x| format!("{:?}", x)).collect();
                write!(f, "[{}]", elements.join(" "))
            }
        }
    }
}

#[derive(Clone)]
pub struct BiteCodeEnv {
    vars: HashMap<String, BiteCodeEvaluated>,
    parent: Option<Rc<RefCell<BiteCodeEnv>>>,
}

impl BiteCodeEnv {
    fn new() -> Self {
        BiteCodeEnv {
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Rc<RefCell<BiteCodeEnv>>) -> Self {
        BiteCodeEnv {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    fn get(&self, name: &str) -> Option<BiteCodeEvaluated> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: BiteCodeEvaluated) {
        self.vars.insert(name, value);
    }
}

#[derive(Clone, Debug)]
pub enum Instruction {
    PushInt(i32),
    Length,
    Add,
    Mult,
    Div,
    Sub,
    Mod,
    Pop,
    Lt,
    Gt,
    Lte,
    Gte,
    Eq,
    And,
    Or,
    Not,

    BitXor,
    BitRs,
    BitLs,
    BitNot,
    BitOr,
    BitAnd,

    StoreVar(String),
    LoadVar(String),
    MakeClosure(Vec<String>, Vec<Instruction>),
    Call(usize),
    MakeArray(usize),
    If {
        then_branch: Vec<Instruction>,
        else_branch: Vec<Instruction>,
    },

    Loop {
        start: Vec<Instruction>,
        end: Vec<Instruction>,
        func: Vec<Instruction>, // code for the lambda expression
    },
    LoopFinish {
        cond: Vec<Instruction>,
        func: Vec<Instruction>,
    },
    SetArray, // expects stack: [value, index, array]
    GetArray,
    PopArray,
}

pub struct VM {
    stack: Vec<BiteCodeEvaluated>,
    locals: Rc<RefCell<BiteCodeEnv>>, // or Rc<RefCell<_>> if needed
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            locals: Rc::new(RefCell::new(BiteCodeEnv::new())),
        }
    }

    pub fn run(&mut self, code: &[Instruction]) {
        for instr in code {
            match instr {
                Instruction::GetArray => {
                    let index_val = self.stack.pop().expect("stack underflow (index)");
                    let array_val = self.stack.pop().expect("stack underflow (array)");

                    match (array_val, index_val) {
                        (BiteCodeEvaluated::Array(arr), BiteCodeEvaluated::Int(i)) => {
                            let r = arr.borrow();
                            if i < 0 || i as usize >= r.len() {
                                panic!("get: index out of bounds");
                            }
                            self.stack.push(r[i as usize].clone());
                        }
                        _ => panic!("get expects (array, int)"),
                    }
                }

                Instruction::Length => {
                    let arr = self
                        .stack
                        .pop()
                        .expect("stack underflow: length needs an array");
                    match arr {
                        BiteCodeEvaluated::Array(elements) => {
                            self.stack
                                .push(BiteCodeEvaluated::Int(elements.borrow().len() as i32));
                        }
                        _ => panic!("length expects an array"),
                    }
                }
                Instruction::PopArray => {
                    let array_val = self.stack.pop().expect("stack underflow");
                    match array_val {
                        BiteCodeEvaluated::Array(arr) => {
                            arr.borrow_mut().pop();
                            self.stack.push(BiteCodeEvaluated::Int(0))
                        }
                        _ => {
                            panic!("pop! argument not an array");
                        }
                    }
                }
                Instruction::SetArray => {
                    // Stack: [..., array(Rc<RefCell<Vec<BiteCodeEvaluated>>>), index(Int), value(BiteCodeEvaluated)]
                    let value = self.stack.pop().expect("stack underflow");
                    let index_val = self.stack.pop().expect("stack underflow");
                    let array_val = self.stack.pop().expect("stack underflow");

                    if let (BiteCodeEvaluated::Array(arr), BiteCodeEvaluated::Int(idx)) =
                        (array_val, index_val)
                    {
                        let len = arr.borrow().len();
                        if idx >= 0 && (idx as usize) <= len {
                            if idx == len as i32 {
                                arr.borrow_mut().push(value);
                            } else {
                                arr.borrow_mut()[idx as usize] = value;
                            }
                        } else {
                            panic!("Index out of bounds");
                        }
                        self.stack.push(BiteCodeEvaluated::Int(0));
                    } else {
                        panic!("set! expects array and integer index");
                    }
                }
                Instruction::If {
                    then_branch,
                    else_branch,
                } => {
                    let cond = self.stack.pop().expect("stack underflow");
                    let cond_val = match cond {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => panic!("if condition must be 0 or 1"),
                    };
                    if cond_val == 1 {
                        self.run(&then_branch);
                    } else {
                        self.run(&else_branch);
                    }
                }
                Instruction::PushInt(n) => self.stack.push(BiteCodeEvaluated::Int(*n)),

                Instruction::Add => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a + b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }

                Instruction::Mult => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a * b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }

                Instruction::Div => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a / b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }

                Instruction::Sub => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a - b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }

                Instruction::Mod => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a % b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }

                Instruction::BitXor => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a ^ b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::BitRs => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a >> b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::BitLs => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a << b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::BitAnd => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a & b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::BitOr => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a | b))
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::BitNot => {
                    let a = self.stack.pop().expect("stack underflow");
                    match (a) {
                        (BiteCodeEvaluated::Int(a)) => self.stack.push(BiteCodeEvaluated::Int(!a)),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Eq => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a == b { 1 } else { 0 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Lt => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a < b { 1 } else { 0 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Gt => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a > b { 1 } else { 0 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Lte => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a <= b { 1 } else { 0 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Gte => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a >= b { 1 } else { 0 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::And => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a == 1 { b } else { 0 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Or => {
                    let b = self.stack.pop().expect("stack underflow");
                    let a = self.stack.pop().expect("stack underflow");
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a == 1 { 1 } else { b })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }
                Instruction::Not => {
                    let a = self.stack.pop().expect("stack underflow");
                    match (a) {
                        (BiteCodeEvaluated::Int(a)) => self
                            .stack
                            .push(BiteCodeEvaluated::Int(if a == 1 { 0 } else { 1 })),
                        _ => panic!("Both arguments must be numbers"),
                    }
                }

                Instruction::Pop => {
                    self.stack.pop().expect("stack underflow");
                }

                Instruction::StoreVar(name) => {
                    let val = self.stack.pop().expect("stack underflow");
                    let mut locals = self.locals.borrow_mut();
                    locals.vars.insert(name.clone(), val);
                }

                Instruction::LoadVar(name) => {
                    let val = self
                        .locals
                        .borrow_mut()
                        .get(name)
                        .expect("undefined variable");
                    self.stack.push(val.clone());
                }

                Instruction::MakeArray(n) => {
                    let mut elements = Vec::new();
                    for _ in 0..*n {
                        elements.push(self.stack.pop().expect("stack underflow"));
                    }
                    elements.reverse(); // preserve order
                    self.stack
                        .push(BiteCodeEvaluated::Array(Rc::new(RefCell::new(elements))));
                }

                Instruction::MakeClosure(params, body) => {
                    let closure = BiteCodeEvaluated::Function(
                        params.clone(),
                        body.clone(),
                        Rc::new(RefCell::new(BiteCodeEnv::with_parent(Rc::clone(
                            &self.locals,
                        )))),
                    );

                    self.stack.push(closure);
                }

                Instruction::Call(arg_count) => {
                    let func = self.stack.pop().expect("stack underflow");
                    let args: Vec<BiteCodeEvaluated> = (0..*arg_count)
                        .map(|_| self.stack.pop().expect("stack underflow"))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();

                    match func {
                        BiteCodeEvaluated::Function(params, body, env) => {
                            if params.len() != args.len() {
                                panic!("Expected {} args, got {}", params.len(), args.len());
                            }

                            // Create a new Env with captured env as parent
                            let local_env =
                                Rc::new(RefCell::new(BiteCodeEnv::with_parent(Rc::clone(&env))));

                            // Bind arguments
                            {
                                let mut local_env_ref = local_env.borrow_mut();
                                for (p, v) in params.iter().zip(args) {
                                    local_env_ref.set(p.clone(), v);
                                    // wrap if needed
                                }
                            }

                            // Run closure body in new VM with local_env
                            let mut inner_vm = VM {
                                stack: Vec::new(),
                                locals: local_env, // store Rc<RefCell<Env>>
                            };
                            inner_vm.run(&body);

                            let result = inner_vm.stack.pop().unwrap_or(BiteCodeEvaluated::Int(0));
                            self.stack.push(result);
                        }
                        _ => panic!("Cannot call non-function"),
                    }
                }
                Instruction::LoopFinish { cond, func } => {
                    loop {
                        // evaluate condition
                        let mut cond_vm = VM {
                            stack: Vec::new(),
                            locals: self.locals.clone(),
                        };
                        cond_vm.run(cond);
                        let cond_val = cond_vm.stack.pop().expect("loop-finish: missing condition");

                        let cond_int = match cond_val {
                            BiteCodeEvaluated::Int(n) => n,
                            _ => panic!("loop-finish condition must evaluate to int"),
                        };

                        if cond_int != 1 {
                            break; // exit loop
                        }

                        // evaluate function expression
                        let mut func_vm = VM {
                            stack: Vec::new(),
                            locals: self.locals.clone(),
                        };
                        func_vm.run(func);
                        let func_val = func_vm.stack.pop().expect("loop-finish: missing function");

                        match func_val {
                            BiteCodeEvaluated::Function(params, body, env) => {
                                if !params.is_empty() {
                                    panic!("loop-finish lambda must take 0 params");
                                }
                                let mut inner_vm = VM {
                                    stack: Vec::new(),
                                    locals: env.clone(),
                                };
                                inner_vm.run(&body);
                            }
                            _ => panic!("loop-finish: second argument must be a lambda"),
                        }
                    }

                    // by convention, return 0
                    self.stack.push(BiteCodeEvaluated::Int(0));
                }
                Instruction::Loop { start, end, func } => {
                    // Evaluate start
                    let mut tmp_start = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    tmp_start.run(start);
                    let start_val = tmp_start.stack.pop().expect("loop: missing start");
                    let start = match start_val {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => panic!("loop: start must be int"),
                    };

                    // Evaluate end
                    let mut tmp_end = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    tmp_end.run(end);
                    let end_val = tmp_end.stack.pop().expect("loop: missing end");
                    let end = match end_val {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => panic!("loop: end must be int"),
                    };

                    // Evaluate function expression
                    let mut tmp_func = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    tmp_func.run(func);
                    let func_val = tmp_func.stack.pop().expect("loop: missing function");

                    let closure = match func_val {
                        BiteCodeEvaluated::Function(params, body, env) => (params, body, env),
                        _ => panic!("loop: third argument must be a lambda"),
                    };

                    // Run iterations
                    for i in start..end {
                        let mut new_locals = closure.2.clone();
                        if closure.1.is_empty() {
                            continue;
                        }
                        // closure must have exactly 1 param
                        if closure.0.len() != 1 {
                            panic!("loop lambda must take exactly 1 param");
                        }
                        new_locals
                            .borrow_mut()
                            .vars
                            .insert(closure.0[0].clone(), BiteCodeEvaluated::Int(i));

                        let mut inner_vm = VM {
                            stack: Vec::new(),
                            locals: new_locals,
                        };
                        inner_vm.run(&closure.1);
                    }

                    // convention: loop returns 0
                    self.stack.push(BiteCodeEvaluated::Int(0));
                }
            }
        }
    }

    pub fn result(&self) -> Option<&BiteCodeEvaluated> {
        self.stack.last()
    }
}

pub fn compile(expr: &Expression, code: &mut Vec<Instruction>) {
    match expr {
        Expression::Atom(n) => {
            code.push(Instruction::PushInt(*n));
        }

        Expression::Word(name) => {
            match name.as_str() {
                "mod" => {
                    // push a closure representing +
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Mod,
                        ],
                    ));
                }
                "+" => {
                    // push a closure representing +
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Add,
                        ],
                    ));
                }
                "-" => {
                    // push a closure representing +
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Sub,
                        ],
                    ));
                }
                "*" => {
                    // push a closure representing +
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Mult,
                        ],
                    ));
                }
                ">" => {
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Gt,
                        ], // you'd need Instruction::Gt
                    ));
                }
                "<" => {
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Lt,
                        ], // you'd need Instruction::Gt
                    ));
                }
                ">=" => {
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Gte,
                        ], // you'd need Instruction::Gt
                    ));
                }
                "<=" => {
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Lte,
                        ], // you'd need Instruction::Gt
                    ));
                }
                "=" => {
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Eq,
                        ], // you'd need Instruction::Gt
                    ));
                }
                "length" => {
                    code.push(Instruction::MakeClosure(
                        vec!["xs".to_string()],
                        vec![Instruction::LoadVar("xs".to_string()), Instruction::Length], // you'd need Instruction::Gt
                    ));
                }
                "not" => {
                    code.push(Instruction::MakeClosure(
                        vec!["a".to_string()],
                        vec![Instruction::LoadVar("a".to_string()), Instruction::Not], // you'd need Instruction::Gt
                    ));
                }
                _ => {
                    code.push(Instruction::LoadVar(name.clone()));
                }
            }
        }

        Expression::Apply(exprs) => {
            if let Expression::Word(op) = &exprs[0] {
                match op.as_str() {
                    "+" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Add);
                    }
                    "*" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Mult);
                    }
                    "/" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Div);
                    }
                    "-" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Sub);
                    }
                    "mod" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Mod);
                    }
                    "=" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Eq);
                    }
                    "<" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Lt);
                    }
                    ">" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Gt);
                    }
                    "<=" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Lte);
                    }
                    ">=" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Gte);
                    }
                    "and" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::And);
                    }
                    "or" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Or);
                    }
                    "not" => {
                        compile(&exprs[1], code);
                        code.push(Instruction::Not);
                    }

                    ">>" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitRs);
                    }
                    "<<" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitLs);
                    }
                    "^" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitXor);
                    }
                    "&" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitAnd);
                    }
                    "|" => {
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitOr);
                    }
                    "~" => {
                        compile(&exprs[1], code);
                        code.push(Instruction::BitNot);
                    }

                    "do" => {
                        for (i, e) in exprs[1..].iter().enumerate() {
                            compile(e, code);
                            if i < exprs.len() - 2 {
                                code.push(Instruction::Pop);
                            }
                        }
                    }
                    "length" => {
                        if exprs.len() != 2 {
                            panic!("length expects exactly 1 argument");
                        }
                        compile(&exprs[1], code); // compile array expression
                        code.push(Instruction::Length);
                    }
                    "let" => {
                        if exprs.len() != 3 {
                            panic!("let requires exactly 2 arguments: name and value");
                        }
                        let var_name = match &exprs[1] {
                            Expression::Word(name) => name.clone(),
                            _ => panic!("let variable must be a word"),
                        };
                        compile(&exprs[2], code); // evaluate value
                        code.push(Instruction::StoreVar(var_name));
                        // push sentinel Unit (here just Int 0) so `do` sees something
                        code.push(Instruction::PushInt(0));
                    }
                    "lambda" => {
                        // args: (lambda param1 param2 ... body)
                        if exprs.len() < 2 {
                            panic!("lambda requires at least 1 param and a body");
                        }

                        let params: Vec<String> = exprs[1..exprs.len() - 1]
                            .iter()
                            .map(|e| match e {
                                Expression::Word(name) => name.clone(),
                                _ => panic!("lambda params must be words"),
                            })
                            .collect();

                        let body_expr = &exprs[exprs.len() - 1];
                        let mut body_code = Vec::new();
                        compile(body_expr, &mut body_code);

                        code.push(Instruction::MakeClosure(params, body_code));
                    }
                    "array" => {
                        let count = exprs.len() - 1;
                        for arg in &exprs[1..] {
                            compile(arg, code);
                        }
                        code.push(Instruction::MakeArray(count));
                    }
                    "if" => {
                        if exprs.len() != 4 {
                            panic!("if requires exactly 3 arguments");
                        }
                        compile(&exprs[1], code); // compile condition

                        let mut then_code = Vec::new();
                        compile(&exprs[2], &mut then_code);

                        let mut else_code = Vec::new();
                        compile(&exprs[3], &mut else_code);

                        code.push(Instruction::If {
                            then_branch: then_code,
                            else_branch: else_code,
                        });
                    }
                    "loop-finish" => {
                        if exprs.len() != 3 {
                            panic!("loop-finish expects 2 arguments: condition, lambda");
                        }

                        let mut cond_code = Vec::new();
                        compile(&exprs[1], &mut cond_code);

                        let mut func_code = Vec::new();
                        compile(&exprs[2], &mut func_code);

                        code.push(Instruction::LoopFinish {
                            cond: cond_code,
                            func: func_code,
                        });
                    }
                    "loop" => {
                        if exprs.len() != 4 {
                            panic!("loop expects 3 arguments: start, end, lambda");
                        }

                        let mut start_code = Vec::new();
                        compile(&exprs[1], &mut start_code);

                        let mut end_code = Vec::new();
                        compile(&exprs[2], &mut end_code);

                        let mut func_code = Vec::new();
                        compile(&exprs[3], &mut func_code);

                        code.push(Instruction::Loop {
                            start: start_code,
                            end: end_code,
                            func: func_code,
                        });
                    }
                    "set!" => {
                        if exprs.len() != 4 {
                            panic!("set! expects 3 arguments: array, index, value");
                        }
                        compile(&exprs[1], code); // array
                        compile(&exprs[2], code); // index
                        compile(&exprs[3], code); // value
                        code.push(Instruction::SetArray);
                    }
                    "pop!" => {
                        if exprs.len() != 2 {
                            panic!("set! expects 1 arguments: array, index, value");
                        }
                        compile(&exprs[1], code); // array
                        code.push(Instruction::PopArray);
                    }

                    "get" => {
                        if exprs.len() != 3 {
                            panic!("get expects 2 arguments: array, index");
                        }
                        // push array
                        compile(&exprs[1], code);
                        // push index
                        compile(&exprs[2], code);
                        // emit get
                        code.push(Instruction::GetArray);
                    }

                    _ => match &exprs[0] {
                        Expression::Word(name) => {
                            // push all arguments first
                            for arg in &exprs[1..] {
                                compile(arg, code);
                            }
                            // load the function/variable and call
                            code.push(Instruction::LoadVar(name.clone()));
                            code.push(Instruction::Call(exprs.len() - 1));
                        }
                        _ => panic!("Cannot call non-word expression"),
                    },
                }
            }
        }
    }
}
