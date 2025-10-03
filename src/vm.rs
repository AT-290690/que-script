use crate::parser::Expression;
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
    Not,

    BitXor,
    BitRs,
    BitLs,
    BitNot,
    BitOr,
    BitAnd,

    StoreVar(String),
    LoadVar(String),
    MakeLambda(Vec<String>, Vec<Instruction>),
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
    SetArray, // expects stack: [value, index, vector]
    GetArray,
    PopArray,
}

impl Instruction {
    pub fn to_rust(&self) -> String {
        match self {
            Instruction::PushInt(n) => format!("PushInt({})", n),
            Instruction::Length => "Length".to_string(),
            Instruction::Add => "Add".to_string(),
            Instruction::Mult => "Mult".to_string(),
            Instruction::Div => "Div".to_string(),
            Instruction::Sub => "Sub".to_string(),
            Instruction::Mod => "Mod".to_string(),
            Instruction::Pop => "Pop".to_string(),
            Instruction::Lt => "Lt".to_string(),
            Instruction::Gt => "Gt".to_string(),
            Instruction::Lte => "Lte".to_string(),
            Instruction::Gte => "Gte".to_string(),
            Instruction::Eq => "Eq".to_string(),
            Instruction::Not => "Not".to_string(),

            Instruction::BitXor => "BitXor".to_string(),
            Instruction::BitRs => "BitRs".to_string(),
            Instruction::BitLs => "BitLs".to_string(),
            Instruction::BitNot => "BitNot".to_string(),
            Instruction::BitOr => "BitOr".to_string(),
            Instruction::BitAnd => "BitAnd".to_string(),

            Instruction::StoreVar(name) => format!("StoreVar(s!({:?}))", name),
            Instruction::LoadVar(name) => format!("LoadVar(s!({:?}))", name),

            Instruction::MakeLambda(params, body) => {
                let params_str = if params.is_empty() {
                    "Vec::<String>::new()".to_string()
                } else {
                    format!(
                        "vec![{}]",
                        params
                            .iter()
                            .map(|s| format!("s!({:?})", s))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                };
                let body_str = body
                    .iter()
                    .map(|instr| instr.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("MakeLambda({}, vec![{}])", params_str, body_str)
            }

            Instruction::Call(n) => format!("Call({})", n),
            Instruction::MakeArray(n) => format!("MakeArray({})", n),

            Instruction::If {
                then_branch,
                else_branch,
            } => {
                let then_str = then_branch
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                let else_str = else_branch
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "If {{ then_branch: vec![{}], else_branch: vec![{}] }}",
                    then_str, else_str
                )
            }

            Instruction::Loop { start, end, func } => {
                let start_str = start
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                let end_str = end
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                let func_str = func
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "Loop {{ start: vec![{}], end: vec![{}], func: vec![{}] }}",
                    start_str, end_str, func_str
                )
            }

            Instruction::LoopFinish { cond, func } => {
                let cond_str = cond
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                let func_str = func
                    .iter()
                    .map(|i| i.to_rust())
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "LoopFinish {{ cond: vec![{}], func: vec![{}] }}",
                    cond_str, func_str
                )
            }

            Instruction::SetArray => "SetArray".to_string(),
            Instruction::GetArray => "GetArray".to_string(),
            Instruction::PopArray => "PopArray".to_string(),
        }
    }
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
                    let array_val = self.stack.pop().expect("stack underflow (vector)");

                    match (array_val, index_val) {
                        (BiteCodeEvaluated::Array(arr), BiteCodeEvaluated::Int(i)) => {
                            let r = arr.borrow();
                            if i < 0 || i as usize >= r.len() {
                                panic!("get: index out of bounds");
                            }
                            self.stack.push(r[i as usize].clone());
                        }
                        _ => panic!("get expects (vector, int)"),
                    }
                }

                Instruction::Length => {
                    let arr = self
                        .stack
                        .pop()
                        .expect("stack underflow: length needs an vector");
                    match arr {
                        BiteCodeEvaluated::Array(elements) => {
                            self.stack
                                .push(BiteCodeEvaluated::Int(elements.borrow().len() as i32));
                        }
                        _ => panic!("length expects an vector"),
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
                            panic!("pop! argument not an vector");
                        }
                    }
                }
                Instruction::SetArray => {
                    // Stack: [..., vector(Rc<RefCell<Vec<BiteCodeEvaluated>>>), index(Int), value(BiteCodeEvaluated)]
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
                        panic!("set! expects vector and integer index");
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

                Instruction::MakeLambda(params, body) => {
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
                    let mut args: Vec<BiteCodeEvaluated> = (0..*arg_count)
                        .map(|_| self.stack.pop().expect("stack underflow"))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();

                    let mut current_func = func;

                    loop {
                        match current_func {
                            BiteCodeEvaluated::Function(params, body, env) => {
                                let consumed = args.len().min(params.len());
                                let (used_args, remaining_args) = args.split_at(consumed);

                                let local_env =
                                    Rc::new(RefCell::new(BiteCodeEnv::with_parent(env.clone())));
                                {
                                    let mut local_env_ref = local_env.borrow_mut();
                                    for (p, v) in
                                        params.iter().take(consumed).zip(used_args.iter().cloned())
                                    {
                                        local_env_ref.set(p.clone(), v);
                                    }
                                }

                                if consumed == params.len() {
                                    // Run body if we've satisfied this function's arguments
                                    let mut inner_vm = VM {
                                        stack: Vec::new(),
                                        locals: local_env,
                                    };
                                    inner_vm.run(&body);

                                    let result =
                                        inner_vm.stack.pop().unwrap_or(BiteCodeEvaluated::Int(0));

                                    if remaining_args.is_empty() {
                                        // No more args to apply -> we're done
                                        self.stack.push(result);
                                        break;
                                    } else {
                                        // More args left, result must be a function -> continue applying
                                        current_func = result;
                                        args = remaining_args.to_vec();
                                        continue;
                                    }
                                } else {
                                    // Partial application -> return closure waiting for the rest
                                    let remaining_params = params[consumed..].to_vec();
                                    self.stack.push(BiteCodeEvaluated::Function(
                                        remaining_params,
                                        body,
                                        local_env,
                                    ));
                                    break;
                                }
                            }
                            _ => panic!("Cannot call non-function"),
                        }
                    }
                }

                Instruction::Loop { start, end, func } => {
                    // Evaluate start
                    let mut start_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    start_vm.run(start);
                    let start_val = start_vm.stack.pop().expect("loop: missing start value");

                    // Evaluate end
                    let mut end_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    end_vm.run(end);
                    let end_val = end_vm.stack.pop().expect("loop: missing end value");

                    let start_int = match start_val {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => panic!("loop start must be int"),
                    };
                    let end_int = match end_val {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => panic!("loop end must be int"),
                    };

                    // Pre-resolve the function ONCE
                    let mut func_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    func_vm.run(func);
                    let func_val = func_vm.stack.pop().expect("loop: missing function");
                    let (params, body, captured_env) = match func_val {
                        BiteCodeEvaluated::Function(p, b, e) => (p, b, e),
                        _ => panic!("loop: third argument must be a lambda"),
                    };

                    if params.len() != 1 {
                        panic!("loop: lambda must take exactly one parameter");
                    }

                    let mut inner_vm = VM {
                        stack: Vec::new(),
                        locals: captured_env.clone(),
                    };

                    for i in start_int..end_int {
                        inner_vm.stack.clear(); // reuse stack
                        inner_vm
                            .locals
                            .borrow_mut()
                            .set(params[0].clone(), BiteCodeEvaluated::Int(i));
                        inner_vm.run(&body);
                    }

                    self.stack.push(BiteCodeEvaluated::Int(0));
                }

                Instruction::LoopFinish { cond, func } => {
                    // Pre-resolve function once (like in Loop)
                    let mut func_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    func_vm.run(func);
                    let func_val = func_vm.stack.pop().expect("loop-finish: missing function");
                    let (params, body, captured_env) = match func_val {
                        BiteCodeEvaluated::Function(p, b, e) => (p, b, e),
                        _ => panic!("loop-finish: second argument must be a lambda"),
                    };

                    if !params.is_empty() {
                        panic!("loop-finish: lambda must take 0 params");
                    }

                    // Reuse the same VMs
                    let mut cond_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };

                    let mut inner_vm = VM {
                        stack: Vec::new(),
                        locals: captured_env.clone(),
                    };

                    loop {
                        cond_vm.stack.clear();
                        cond_vm.run(cond);

                        let cond_val = cond_vm.stack.pop().expect("loop-finish: missing condition");
                        let cond_int = match cond_val {
                            BiteCodeEvaluated::Int(n) => n,
                            _ => panic!("loop-finish condition must be int"),
                        };

                        if cond_int != 1 {
                            break;
                        }

                        inner_vm.stack.clear();
                        inner_vm.run(&body);
                    }

                    self.stack.push(BiteCodeEvaluated::Int(0)); // by convention
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
                // push a closure representing these
                "/" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Div,
                        ],
                    ));
                }
                "mod" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Mod,
                        ],
                    ));
                }
                "+" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Add,
                        ],
                    ));
                }
                "-" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Sub,
                        ],
                    ));
                }
                "*" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Mult,
                        ],
                    ));
                }
                ">" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Gt,
                        ],
                    ));
                }
                "<" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Lt,
                        ],
                    ));
                }
                ">=" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Gte,
                        ],
                    ));
                }
                "<=" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Lte,
                        ],
                    ));
                }
                "=" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Eq,
                        ],
                    ));
                }
                "length" => {
                    code.push(Instruction::MakeLambda(
                        vec!["xs".to_string()],
                        vec![Instruction::LoadVar("xs".to_string()), Instruction::Length],
                    ));
                }
                "not" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string()],
                        vec![Instruction::LoadVar("a".to_string()), Instruction::Not],
                    ));
                }

                ">>" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::BitRs,
                        ],
                    ));
                }
                "<<" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::BitLs,
                        ],
                    ));
                }
                "^" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::BitXor,
                        ],
                    ));
                }
                "|" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::BitOr,
                        ],
                    ));
                }
                "&" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::BitAnd,
                        ],
                    ));
                }
                "~" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string()],
                        vec![Instruction::LoadVar("a".to_string()), Instruction::BitNot],
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
                        if exprs.len() != 3 {
                            panic!("+ expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Add);
                    }
                    "*" => {
                        if exprs.len() != 3 {
                            panic!("* expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Mult);
                    }
                    "/" => {
                        if exprs.len() != 3 {
                            panic!("/ expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Div);
                    }
                    "-" => {
                        if exprs.len() != 3 {
                            panic!("- expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Sub);
                    }
                    "mod" => {
                        if exprs.len() != 3 {
                            panic!("mod expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Mod);
                    }
                    "=" => {
                        if exprs.len() != 3 {
                            panic!("= expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Eq);
                    }
                    "<" => {
                        if exprs.len() != 3 {
                            panic!("< expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Lt);
                    }
                    ">" => {
                        if exprs.len() != 3 {
                            panic!("> expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Gt);
                    }
                    "<=" => {
                        if exprs.len() != 3 {
                            panic!("<= expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Lte);
                    }
                    ">=" => {
                        if exprs.len() != 3 {
                            panic!(">= expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::Gte);
                    }
                    "and" => {
                        if exprs.len() != 3 {
                            panic!("and expects exactly 2 arguments");
                        }

                        let mut then_code = Vec::new();
                        compile(&exprs[2], &mut then_code);
                        // First argument is the condition
                        compile(&exprs[1], code);
                        code.push(Instruction::If {
                            then_branch: then_code,
                            else_branch: vec![Instruction::PushInt(0)],
                        });
                    }
                    "or" => {
                        if exprs.len() != 3 {
                            panic!("or expects exactly 2 arguments");
                        }

                        let mut then_code = vec![Instruction::PushInt(1)];
                        compile(&exprs[2], &mut then_code);

                        compile(&exprs[1], code);
                        code.push(Instruction::If {
                            then_branch: vec![Instruction::PushInt(1)],
                            else_branch: then_code,
                        });
                    }

                    "not" => {
                        if exprs.len() != 2 {
                            panic!("not expects exactly 1 arguments");
                        }
                        compile(&exprs[1], code);
                        code.push(Instruction::Not);
                    }

                    ">>" => {
                        if exprs.len() != 3 {
                            panic!(">> expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitRs);
                    }
                    "<<" => {
                        if exprs.len() != 3 {
                            panic!("<< expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitLs);
                    }
                    "^" => {
                        if exprs.len() != 3 {
                            panic!("^ expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitXor);
                    }
                    "&" => {
                        if exprs.len() != 3 {
                            panic!("& expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitAnd);
                    }
                    "|" => {
                        if exprs.len() != 3 {
                            panic!("| expects exactly 2 arguments");
                        }
                        compile(&exprs[1], code);
                        compile(&exprs[2], code);
                        code.push(Instruction::BitOr);
                    }
                    "~" => {
                        if exprs.len() != 2 {
                            panic!("~ expects exactly 1 arguments");
                        }
                        compile(&exprs[1], code);
                        code.push(Instruction::BitNot);
                    }

                    "do" => {
                        if exprs.len() <= 0 {
                            panic!("do expects atleast 1 argument");
                        }
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
                        compile(&exprs[1], code); // compile vector expression
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

                        code.push(Instruction::MakeLambda(params, body_code));
                    }
                    "as" => {
                        code.push(Instruction::MakeArray(0));
                    }
                    "vector" => {
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
                            panic!("set! expects 3 arguments: vector, index, value");
                        }
                        compile(&exprs[1], code); // vector
                        compile(&exprs[2], code); // index
                        compile(&exprs[3], code); // value
                        code.push(Instruction::SetArray);
                    }
                    "pop!" => {
                        if exprs.len() != 2 {
                            panic!("set! expects 1 arguments: vector, index, value");
                        }
                        compile(&exprs[1], code); // vector
                        code.push(Instruction::PopArray);
                    }

                    "get" => {
                        if exprs.len() != 3 {
                            panic!("get expects 2 arguments: vector, index");
                        }
                        // push vector
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

pub fn run(expr: &crate::parser::Expression) -> BiteCodeEvaluated {
    let mut code = Vec::new();
    compile(&expr, &mut code);
    let mut vm = VM::new();
    vm.run(&code);
    return vm.result().unwrap_or(&BiteCodeEvaluated::Int(0)).clone();
}

pub fn exe(code: Vec<Instruction>) -> BiteCodeEvaluated {
    let mut vm = VM::new();
    vm.run(&code);
    return vm.result().unwrap_or(&BiteCodeEvaluated::Int(0)).clone();
}
