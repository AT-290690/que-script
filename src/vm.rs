use crate::parser::Expression;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};
#[derive(Clone)]
pub enum BiteCodeEvaluated {
    Bool(bool),
    Int(i32),
    Function(Vec<String>, Vec<Instruction>, Rc<RefCell<BiteCodeEnv>>),
    Array(Rc<RefCell<Vec<BiteCodeEvaluated>>>),
}

impl fmt::Debug for BiteCodeEvaluated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BiteCodeEvaluated::Bool(value) => write!(f, "{}", value),
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
    parent: Option<Weak<RefCell<BiteCodeEnv>>>,
}

impl BiteCodeEnv {
    fn new() -> Self {
        BiteCodeEnv {
            vars: HashMap::with_capacity(16), // Pre-allocate space for common variables
            parent: None,
        }
    }

    fn with_parent(parent: Rc<RefCell<BiteCodeEnv>>) -> Self {
        BiteCodeEnv {
            vars: HashMap::with_capacity(16), // Pre-allocate space for common variables
            parent: Some(Rc::downgrade(&parent)),
        }
    }

    fn get(&self, name: &str) -> Option<BiteCodeEvaluated> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.clone());
        }
        if let Some(ref weak_parent) = self.parent {
            return weak_parent.upgrade()?.borrow().get(name);
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
    PushBool(bool),

    StoreVar(String),
    LoadVar(String),
    MakeLambda(Vec<String>, Vec<Instruction>),
    Call(usize),
    MakeVector(usize),
    If(Vec<Instruction>, Vec<Instruction>),

    Loop(
        Vec<Instruction>,
        Vec<Instruction>,
        Vec<Instruction>, // code for the lambda expression
    ),
    LoopFinish(Vec<Instruction>, Vec<Instruction>),

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

    EqBool,

    BitXor,
    BitRs,
    BitLs,
    BitNot,
    BitOr,
    BitAnd,

    SetArray, // expects stack: [value,index,vector]
    GetArray,
    PopArray,
}

impl Instruction {
    pub fn to_rust(&self) -> String {
        match self {
            Instruction::PushBool(n) => format!("PushBool({})", n),
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
            Instruction::EqBool => "EqBool".to_string(),

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
                format!("MakeLambda({},vec![{}])", params_str, body_str)
            }

            Instruction::Call(n) => format!("Call({})", n),
            Instruction::MakeVector(n) => format!("MakeVector({})", n),

            Instruction::If(then_branch, else_branch) => {
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
                format!("If(vec![{}],vec![{}])", then_str, else_str)
            }

            Instruction::Loop(start, end, func) => {
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
                    "Loop(vec![{}],vec![{}],vec![{}])",
                    start_str, end_str, func_str
                )
            }

            Instruction::LoopFinish(cond, func) => {
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
                format!("LoopFinish(vec![{}],vec![{}])", cond_str, func_str)
            }

            Instruction::SetArray => "SetArray".to_string(),
            Instruction::GetArray => "GetArray".to_string(),
            Instruction::PopArray => "PopArray".to_string(),
        }
    }
    pub fn serialise(&self) -> String {
        match self {
            Instruction::PushBool(n) => format!("PushBool({})", n),
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
            Instruction::EqBool => "EqBool".to_string(),

            Instruction::Not => "Not".to_string(),

            Instruction::BitXor => "BitXor".to_string(),
            Instruction::BitRs => "BitRs".to_string(),
            Instruction::BitLs => "BitLs".to_string(),
            Instruction::BitNot => "BitNot".to_string(),
            Instruction::BitOr => "BitOr".to_string(),
            Instruction::BitAnd => "BitAnd".to_string(),

            Instruction::StoreVar(name) => format!("StoreVar({:?})", name),
            Instruction::LoadVar(name) => format!("LoadVar({:?})", name),

            Instruction::MakeLambda(params, body) => {
                let params_str = if params.is_empty() {
                    "[]".to_string()
                } else {
                    format!(
                        "[{}]",
                        params
                            .iter()
                            .map(|s| format!("{:?}", s))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                };
                let body_str = body
                    .iter()
                    .map(|instr| instr.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("MakeLambda({},[{}])", params_str, body_str)
            }

            Instruction::Call(n) => format!("Call({})", n),
            Instruction::MakeVector(n) => format!("MakeVector({})", n),

            Instruction::If(then_branch, else_branch) => {
                let then_str = then_branch
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                let else_str = else_branch
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("If([{}],[{}])", then_str, else_str)
            }

            Instruction::Loop(start, end, func) => {
                let start_str = start
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                let end_str = end
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                let func_str = func
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("Loop([{}],[{}],[{}])", start_str, end_str, func_str)
            }

            Instruction::LoopFinish(cond, func) => {
                let cond_str = cond
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                let func_str = func
                    .iter()
                    .map(|i| i.serialise())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("LoopFinish([{}],[{}])", cond_str, func_str)
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
            stack: Vec::with_capacity(64), // Pre-allocate stack space
            locals: Rc::new(RefCell::new(BiteCodeEnv::new())),
        }
    }

    pub fn run(&mut self, code: &[Instruction]) -> Result<(), String> {
        const LOOP_LIMIT: i32 = 5000000;
        let mut universal_loop_count = 0;
        for instr in code {
            match instr {
                Instruction::GetArray => {
                    let index_val = self.stack.pop().ok_or("stack underflow (index)")?;
                    let array_val = self.stack.pop().ok_or("stack underflow (vector)")?;
                    match (array_val, index_val) {
                        (BiteCodeEvaluated::Array(arr), BiteCodeEvaluated::Int(i)) => {
                            let r = arr.borrow();
                            if i < 0 || i as usize >= r.len() {
                                return Err("Error! get: index out of bounds".to_string());
                            }
                            self.stack.push(r[i as usize].clone());
                        }
                        _ => return Err("Error! get expects (vector,int)".to_string()),
                    }
                }

                Instruction::Length => {
                    let arr = self
                        .stack
                        .pop()
                        .ok_or("stack underflow: length needs an vector")?;
                    match arr {
                        BiteCodeEvaluated::Array(elements) => {
                            self.stack
                                .push(BiteCodeEvaluated::Int(elements.borrow().len() as i32));
                        }
                        _ => return Err("Error! length expects an vector".to_string()),
                    }
                }
                Instruction::PopArray => {
                    let array_val = self.stack.pop().ok_or("stack underflow")?;
                    match array_val {
                        BiteCodeEvaluated::Array(arr) => {
                            arr.borrow_mut().pop();
                            self.stack.push(BiteCodeEvaluated::Int(0))
                        }
                        _ => {
                            return Err("Error! pop! argument not an vector".to_string());
                        }
                    }
                }
                Instruction::SetArray => {
                    // Stack: [...,vector(Rc<RefCell<Vec<BiteCodeEvaluated>>>),index(Int),value(BiteCodeEvaluated)]
                    let value = self.stack.pop().ok_or("stack underflow")?;
                    let index_val = self.stack.pop().ok_or("stack underflow")?;
                    let array_val = self.stack.pop().ok_or("stack underflow")?;

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
                            return Err("Error! Index out of bounds".to_string());
                        }
                        self.stack.push(BiteCodeEvaluated::Int(0));
                    } else {
                        return Err("Error! set! expects vector and integer index".to_string());
                    }
                }
                Instruction::If(then_branch, else_branch) => {
                    let cond = self.stack.pop().ok_or("stack underflow")?;
                    let cond_val = match cond {
                        BiteCodeEvaluated::Bool(n) => n,
                        _ => return Err("Error! if condition must be true or false".to_string()),
                    };
                    if cond_val {
                        self.run(&then_branch)?;
                    } else {
                        self.run(&else_branch)?;
                    }
                }
                Instruction::PushInt(n) => self.stack.push(BiteCodeEvaluated::Int(*n)),
                Instruction::PushBool(n) => self.stack.push(BiteCodeEvaluated::Bool(*n)),

                Instruction::Add => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a + b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (+)".to_string()),
                    }
                }

                Instruction::Mult => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a * b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (*)".to_string()),
                    }
                }

                Instruction::Div => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a / b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (/)".to_string()),
                    }
                }

                Instruction::Sub => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a - b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (-)".to_string()),
                    }
                }

                Instruction::Mod => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a % b))
                        }
                        _ => {
                            return Err("Error! Both arguments must be numbers at (mod)".to_string())
                        }
                    }
                }

                Instruction::BitXor => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a ^ b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (^)".to_string()),
                    }
                }
                Instruction::BitRs => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a >> b))
                        }
                        _ => {
                            return Err("Error! Both arguments must be numbers at (>>)".to_string())
                        }
                    }
                }
                Instruction::BitLs => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a << b))
                        }
                        _ => {
                            return Err("Error! Both arguments must be numbers at (<<)".to_string())
                        }
                    }
                }
                Instruction::BitAnd => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a & b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (&)".to_string()),
                    }
                }
                Instruction::BitOr => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Int(a | b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (|)".to_string()),
                    }
                }
                Instruction::BitNot => {
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a) {
                        (BiteCodeEvaluated::Int(a)) => self.stack.push(BiteCodeEvaluated::Int(!a)),
                        _ => return Err("Error! Arguments must be a number at (~)".to_string()),
                    }
                }
                Instruction::Eq => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;

                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(a == b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (=)".to_string()),
                    }
                }
                Instruction::EqBool => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;

                    match (a, b) {
                        (BiteCodeEvaluated::Bool(a), BiteCodeEvaluated::Bool(b)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(a == b))
                        }
                        _ => return Err("Error! Both arguments must be bools at (=)".to_string()),
                    }
                }
                Instruction::Lt => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(a < b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (<)".to_string()),
                    }
                }
                Instruction::Gt => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(a > b))
                        }
                        _ => return Err("Error! Both arguments must be numbers at (>)".to_string()),
                    }
                }
                Instruction::Lte => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(a <= b))
                        }
                        _ => {
                            return Err("Error! Both arguments must be numbers at (<=)".to_string())
                        }
                    }
                }
                Instruction::Gte => {
                    let b = self.stack.pop().ok_or("stack underflow")?;
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a, b) {
                        (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(a >= b))
                        }
                        _ => {
                            return Err("Error! Both arguments must be numbers at (>=)".to_string())
                        }
                    }
                }
                Instruction::Not => {
                    let a = self.stack.pop().ok_or("stack underflow")?;
                    match (a) {
                        (BiteCodeEvaluated::Bool(a)) => {
                            self.stack.push(BiteCodeEvaluated::Bool(!a))
                        }
                        _ => return Err("Error! Argument must be a number at (not)".to_string()),
                    }
                }

                Instruction::Pop => {
                    self.stack.pop().ok_or("stack underflow")?;
                }

                Instruction::StoreVar(name) => {
                    let val = self.stack.pop().ok_or("stack underflow")?;
                    let mut locals = self.locals.borrow_mut();
                    locals.vars.insert(name.clone(), val);
                }

                Instruction::LoadVar(name) => {
                    let val = self
                        .locals
                        .borrow()
                        .get(name)
                        .ok_or(format!("undefined variable: {}", name))?;
                    self.stack.push(val);
                }

                Instruction::MakeVector(n) => {
                    let mut elements = Vec::with_capacity(*n);
                    for _ in 0..*n {
                        elements.push(self.stack.pop().ok_or("stack underflow")?);
                    }
                    elements.reverse(); // preserve order
                    self.stack
                        .push(BiteCodeEvaluated::Array(Rc::new(RefCell::new(elements))));
                }

                Instruction::MakeLambda(params, body) => {
                    // this solution will capture inner closure scope but at the cost of a memory leak!
                    // let closure = BiteCodeEvaluated::Function(
                    //     params.clone(),
                    //     body.clone(),
                    //     Rc::clone(&self.locals), // shared mutable environment
                    // );
                    // self.stack.push(closure);
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
                    let func = self.stack.pop().ok_or("Error! Runtime stack underflow")?;
                    let mut args: Vec<BiteCodeEvaluated> = (0..*arg_count)
                        .map(|_| self.stack.pop().ok_or("Error! Runtime stack underflow"))
                        .collect::<Result<Vec<_>, _>>()?
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
                                    for (p, v) in params.iter().take(consumed).zip(used_args.iter())
                                    {
                                        local_env_ref.set(p.clone(), v.clone());
                                    }
                                }

                                if consumed == params.len() {
                                    // Run body if we've satisfied this function's arguments
                                    // Save current environment and stack
                                    let old_env = self.locals.clone();
                                    let old_stack_len = self.stack.len();

                                    // Switch to function environment
                                    self.locals = local_env;

                                    // Run function body
                                    self.run(&body)?;

                                    // Get result
                                    let result =
                                        self.stack.pop().unwrap_or(BiteCodeEvaluated::Int(0));

                                    // Restore environment and stack
                                    self.locals = old_env;
                                    // Remove any extra stack elements that might have been added
                                    self.stack.truncate(old_stack_len);

                                    if remaining_args.is_empty() {
                                        // No more args to apply -> we're done
                                        self.stack.push(result);
                                        break;
                                    } else {
                                        // More args left,result must be a function -> continue applying
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
                            _ => return Err("Error! Cannot call non-function".to_string()),
                        }
                    }
                }

                Instruction::Loop(start, end, func) => {
                    // Evaluate start
                    let mut start_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    start_vm.run(start)?;
                    let start_val = start_vm
                        .stack
                        .pop()
                        .ok_or("Error! Runtime loop: missing start value")?;

                    // Evaluate end
                    let mut end_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    end_vm.run(end)?;
                    let end_val = end_vm
                        .stack
                        .pop()
                        .ok_or("Error! Runtime loop: missing end value")?;

                    let start_int = match start_val {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => return Err("Error! loop start must be int".to_string()),
                    };
                    let end_int = match end_val {
                        BiteCodeEvaluated::Int(n) => n,
                        _ => return Err("Error! loop end must be int".to_string()),
                    };
                    if end_int > LOOP_LIMIT {
                        return Err("Error! loop iteration is higher than the limit".to_string());
                    }
                    universal_loop_count += end_int;
                    if universal_loop_count > LOOP_LIMIT {
                        return Err("Error! Program is looping too much".to_string());
                    }
                    // Pre-resolve the function ONCE
                    let mut func_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    func_vm.run(func)?;
                    let func_val = func_vm
                        .stack
                        .pop()
                        .ok_or("Error! Runtime loop: missing function")?;
                    let (params, body, captured_env) = match func_val {
                        BiteCodeEvaluated::Function(p, b, e) => (p, b, e),
                        _ => return Err("Error! loop: third argument must be a lambda".to_string()),
                    };

                    if params.len() != 1 {
                        return Err(
                            "Error! loop: lambda must take exactly one parameter".to_string()
                        );
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
                        inner_vm.run(&body)?;
                    }

                    self.stack.push(BiteCodeEvaluated::Int(0));
                }

                Instruction::LoopFinish(cond, func) => {
                    // Pre-resolve function once (like in Loop)
                    let mut func_vm = VM {
                        stack: Vec::new(),
                        locals: self.locals.clone(),
                    };
                    func_vm.run(func)?;
                    let func_val = func_vm
                        .stack
                        .pop()
                        .ok_or("Error! Runtime loop-finish: missing function")?;
                    let (params, body, captured_env) = match func_val {
                        BiteCodeEvaluated::Function(p, b, e) => (p, b, e),
                        _ => {
                            return Err(
                                "Error! loop-finish: second argument must be a lambda".to_string()
                            )
                        }
                    };

                    if !params.is_empty() {
                        return Err("Error! loop-finish: lambda must take 0 params".to_string());
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
                    let mut loop_count = 0;

                    loop {
                        loop_count += 1;
                        if loop_count > LOOP_LIMIT {
                            return Err("Error! loop-finish reached loop limit".to_string());
                        }
                        cond_vm.stack.clear();
                        cond_vm.run(cond)?;

                        let cond_val = cond_vm
                            .stack
                            .pop()
                            .ok_or("Error! Runtime loop-finish: missing condition")?;
                        let cond_bool = match cond_val {
                            BiteCodeEvaluated::Bool(n) => n,
                            _ => {
                                return Err("Error! loop-finish condition must be bool".to_string())
                            }
                        };

                        if !cond_bool {
                            break;
                        }

                        inner_vm.stack.clear();
                        inner_vm.run(&body)?;
                    }
                    universal_loop_count += loop_count;
                    if universal_loop_count > LOOP_LIMIT {
                        return Err("Error! Program is looping too much".to_string());
                    }
                    self.stack.push(BiteCodeEvaluated::Int(0)); // by convention
                }
            }
        }
        Ok(())
    }

    pub fn result(&self) -> Option<&BiteCodeEvaluated> {
        self.stack.last()
    }
}

pub fn compile(expr: &Expression, code: &mut Vec<Instruction>) -> Result<(), String> {
    match expr {
        Expression::Atom(n) => {
            code.push(Instruction::PushInt(*n));
            Ok(())
        }

        Expression::Word(name) => {
            match name.as_str() {
                // TODO add get fst snd and other missing stuff from here
                "true" => {
                    code.push(Instruction::PushBool(true));
                    Ok(())
                }
                "false" => {
                    code.push(Instruction::PushBool(false));
                    Ok(())
                }
                // push a closure representing these
                "/" | "/#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Div,
                        ],
                    ));
                    Ok(())
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
                    Ok(())
                }
                "+" | "+#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Add,
                        ],
                    ));
                    Ok(())
                }
                "-" | "-#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Sub,
                        ],
                    ));
                    Ok(())
                }
                "*" | "*#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Mult,
                        ],
                    ));
                    Ok(())
                }
                ">" | ">#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Gt,
                        ],
                    ));
                    Ok(())
                }
                "<" | "<#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Lt,
                        ],
                    ));
                    Ok(())
                }
                ">=" | ">=#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Gte,
                        ],
                    ));
                    Ok(())
                }
                "<=" | "<=#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Lte,
                        ],
                    ));
                    Ok(())
                }
                "=?" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::EqBool,
                        ],
                    ));
                    Ok(())
                }
                "=" | "=#" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string(), "b".to_string()],
                        vec![
                            Instruction::LoadVar("a".to_string()),
                            Instruction::LoadVar("b".to_string()),
                            Instruction::Eq,
                        ],
                    ));
                    Ok(())
                }
                "length" => {
                    code.push(Instruction::MakeLambda(
                        vec!["xs".to_string()],
                        vec![Instruction::LoadVar("xs".to_string()), Instruction::Length],
                    ));
                    Ok(())
                }
                "not" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string()],
                        vec![Instruction::LoadVar("a".to_string()), Instruction::Not],
                    ));
                    Ok(())
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
                    Ok(())
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
                    Ok(())
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
                    Ok(())
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
                    Ok(())
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
                    Ok(())
                }
                "~" => {
                    code.push(Instruction::MakeLambda(
                        vec!["a".to_string()],
                        vec![Instruction::LoadVar("a".to_string()), Instruction::BitNot],
                    ));
                    Ok(())
                }
                _ => {
                    code.push(Instruction::LoadVar(name.clone()));
                    Ok(())
                }
            }
        }

        Expression::Apply(exprs) => {
            if let Expression::Word(op) = &exprs[0] {
                match op.as_str() {
                    "+" | "+#" => {
                        if exprs.len() != 3 {
                            return Err("Error! + expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Add);
                        Ok(())
                    }
                    "*" | "*#" => {
                        if exprs.len() != 3 {
                            return Err("Error! * expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Mult);
                        Ok(())
                    }
                    "/" | "/#" => {
                        if exprs.len() != 3 {
                            return Err("Error! / expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Div);
                        Ok(())
                    }
                    "-" | "-#" => {
                        if exprs.len() != 3 {
                            return Err("Error! - expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Sub);
                        Ok(())
                    }
                    "mod" => {
                        if exprs.len() != 3 {
                            return Err("Error! mod expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Mod);
                        Ok(())
                    }
                    "=?" => {
                        if exprs.len() != 3 {
                            return Err("Error! = expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::EqBool);
                        Ok(())
                    }
                    "=" | "=#" => {
                        if exprs.len() != 3 {
                            return Err("Error! = expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Eq);
                        Ok(())
                    }
                    "<" | "<#" => {
                        if exprs.len() != 3 {
                            return Err("Error! < expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Lt);
                        Ok(())
                    }
                    ">" | ">#" => {
                        if exprs.len() != 3 {
                            return Err("Error! > expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Gt);
                        Ok(())
                    }
                    "<=" | "<=#" => {
                        if exprs.len() != 3 {
                            return Err("Error! <= expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Lte);
                        Ok(())
                    }
                    ">=" | ">=#" => {
                        if exprs.len() != 3 {
                            return Err("Error! >= expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Gte);
                        Ok(())
                    }
                    "and" => {
                        if exprs.len() != 3 {
                            return Err("Error! and expects exactly 2 arguments".to_string());
                        }

                        let mut then_code = Vec::new();
                        compile(&exprs[2], &mut then_code)?;
                        // First argument is the condition
                        compile(&exprs[1], code)?;
                        code.push(Instruction::If(
                            then_code,
                            vec![Instruction::PushBool(false)],
                        ));
                        Ok(())
                    }
                    "or" => {
                        if exprs.len() != 3 {
                            return Err("Error! or expects exactly 2 arguments".to_string());
                        }

                        // Evaluate second argument (b)
                        let mut else_code = Vec::new();
                        compile(&exprs[2], &mut else_code)?;

                        // Evaluate first argument (a)
                        compile(&exprs[1], code)?;
                        // If a is true, return true immediately; otherwise, evaluate b
                        code.push(Instruction::If(
                            vec![Instruction::PushBool(true)], // then branch
                            else_code,                         // else branch
                        ));

                        Ok(())
                    }
                    "not" => {
                        if exprs.len() != 2 {
                            return Err("Error! not expects exactly 1 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        code.push(Instruction::Not);
                        Ok(())
                    }

                    ">>" => {
                        if exprs.len() != 3 {
                            return Err("Error! >> expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitRs);
                        Ok(())
                    }
                    "<<" => {
                        if exprs.len() != 3 {
                            return Err("Error! << expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitLs);
                        Ok(())
                    }
                    "^" => {
                        if exprs.len() != 3 {
                            return Err("Error! ^ expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitXor);
                        Ok(())
                    }
                    "&" => {
                        if exprs.len() != 3 {
                            return Err("Error! & expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitAnd);
                        Ok(())
                    }
                    "|" => {
                        if exprs.len() != 3 {
                            return Err("Error! | expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitOr);
                        Ok(())
                    }
                    "~" => {
                        if exprs.len() != 2 {
                            return Err("Error! ~ expects exactly 1 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        code.push(Instruction::BitNot);
                        Ok(())
                    }

                    "do" => {
                        if exprs.len() <= 0 {
                            return Err("Error! do expects atleast 1 argument".to_string());
                        }
                        for (i, e) in exprs[1..].iter().enumerate() {
                            compile(e, code)?;
                            if i < exprs.len() - 2 {
                                code.push(Instruction::Pop);
                            }
                        }
                        Ok(())
                    }
                    "length" => {
                        if exprs.len() != 2 {
                            return Err("Error! length expects exactly 1 argument".to_string());
                        }
                        compile(&exprs[1], code)?; // compile vector expression
                        code.push(Instruction::Length);
                        Ok(())
                    }
                    "let" => {
                        if exprs.len() != 3 {
                            return Err(
                                "let requires exactly 2 arguments: name and value".to_string()
                            );
                        }
                        let var_name = match &exprs[1] {
                            Expression::Word(name) => name.clone(),
                            _ => return Err("Error! let variable must be a word".to_string()),
                        };
                        compile(&exprs[2], code)?; // evaluate value
                        code.push(Instruction::StoreVar(var_name));
                        // push sentinel Unit (here just Int 0) so `do` sees something
                        code.push(Instruction::PushInt(0));
                        Ok(())
                    }
                    "lambda" => {
                        // args: (lambda param1 param2 ... body)
                        if exprs.len() < 2 {
                            return Err(
                                "Error! lambda requires at least 1 param and a body".to_string()
                            );
                        }

                        let mut params = Vec::new();
                        for e in &exprs[1..exprs.len() - 1] {
                            match e {
                                Expression::Word(name) => params.push(name.clone()),
                                _ => return Err("Error! lambda params must be words".to_string()),
                            }
                        }

                        let body_expr = &exprs[exprs.len() - 1];
                        let mut body_code = Vec::new();
                        compile(body_expr, &mut body_code)?;

                        code.push(Instruction::MakeLambda(params, body_code));
                        Ok(())
                    }
                    "as" => {
                        // Just ensure syntax correctness — (as <expr> <type>)
                        if exprs.len() != 3 {
                            return Err(
                                "Error! `as` expects two arguments: (as expr Type)".to_string()
                            );
                        }

                        // Compile the first argument normally
                        compile(&exprs[1], code)?;

                        // We intentionally do NOT emit anything for the type annotation.
                        // The type info is only used by the type checker / inference layer.

                        Ok(())
                    }
                    "char" => {
                        for arg in &exprs[1..] {
                            compile(arg, code)?;
                        }
                        Ok(())
                    }
                    "vector" | "string" | "tuple" => {
                        let count = exprs.len() - 1;
                        for arg in &exprs[1..] {
                            compile(arg, code)?;
                        }
                        code.push(Instruction::MakeVector(count));
                        Ok(())
                    }
                    "if" => {
                        if exprs.len() != 4 {
                            return Err("Error! if requires exactly 3 arguments".to_string());
                        }
                        compile(&exprs[1], code)?; // compile condition

                        let mut then_code = Vec::new();
                        compile(&exprs[2], &mut then_code)?;

                        let mut else_code = Vec::new();
                        compile(&exprs[3], &mut else_code)?;

                        code.push(Instruction::If(then_code, else_code));
                        Ok(())
                    }
                    "loop-finish" => {
                        if exprs.len() != 3 {
                            return Err(
                                "loop-finish expects 2 arguments: condition,lambda".to_string()
                            );
                        }

                        let mut cond_code = Vec::new();
                        compile(&exprs[1], &mut cond_code)?;

                        let mut func_code = Vec::new();
                        compile(&exprs[2], &mut func_code)?;

                        code.push(Instruction::LoopFinish(cond_code, func_code));
                        Ok(())
                    }
                    "loop" => {
                        if exprs.len() != 4 {
                            return Err(
                                "Error! loop expects 3 arguments: start,end,lambda".to_string()
                            );
                        }

                        let mut start_code = Vec::new();
                        compile(&exprs[1], &mut start_code)?;

                        let mut end_code = Vec::new();
                        compile(&exprs[2], &mut end_code)?;

                        let mut func_code = Vec::new();
                        compile(&exprs[3], &mut func_code)?;

                        code.push(Instruction::Loop(start_code, end_code, func_code));
                        Ok(())
                    }
                    "set!" => {
                        if exprs.len() != 4 {
                            return Err(
                                "Error! set! expects 3 arguments: vector,index,value".to_string()
                            );
                        }
                        compile(&exprs[1], code)?; // vector
                        compile(&exprs[2], code)?; // index
                        compile(&exprs[3], code)?; // value
                        code.push(Instruction::SetArray);
                        Ok(())
                    }
                    "pop!" => {
                        if exprs.len() != 2 {
                            return Err("Error! pop! expects 1 argument: vector".to_string());
                        }
                        compile(&exprs[1], code)?; // vector
                        code.push(Instruction::PopArray);
                        Ok(())
                    }

                    "get" => {
                        if exprs.len() != 3 {
                            return Err("Error! get expects 2 arguments: vector,index".to_string());
                        }
                        // push vector
                        compile(&exprs[1], code)?;
                        // push index
                        compile(&exprs[2], code)?;
                        // emit get
                        code.push(Instruction::GetArray);
                        Ok(())
                    }

                    "fst" => {
                        if exprs.len() != 2 {
                            return Err("Error! fst expects 1 arguments: tuple".to_string());
                        }
                        // push vector
                        compile(&exprs[1], code)?;
                        // push index
                        code.push(Instruction::PushInt(0));
                        // emit get
                        code.push(Instruction::GetArray);
                        Ok(())
                    }
                    "snd" => {
                        if exprs.len() != 2 {
                            return Err("Error! snd expects 1 arguments: tuple".to_string());
                        }
                        // push vector
                        compile(&exprs[1], code)?;
                        // push index
                        code.push(Instruction::PushInt(1));
                        // emit get
                        code.push(Instruction::GetArray);
                        Ok(())
                    }
                    _ => match &exprs[0] {
                        Expression::Word(name) => {
                            // push all arguments first
                            for arg in &exprs[1..] {
                                compile(arg, code)?;
                            }
                            // load the function/variable and call
                            code.push(Instruction::LoadVar(name.clone()));
                            code.push(Instruction::Call(exprs.len() - 1));
                            Ok(())
                        }
                        _ => return Err("Error! Cannot call non-word expression".to_string()),
                    },
                }
            } else {
                Ok(())
            }
        }
    }
}

pub fn run(expr: &crate::parser::Expression) -> Result<BiteCodeEvaluated, String> {
    let mut code = Vec::new();
    compile(&expr, &mut code)?;
    let mut vm = VM::new();
    vm.run(&code)?;
    return Ok(vm.result().unwrap_or(&BiteCodeEvaluated::Int(0)).clone());
}

pub fn exe(code: Vec<Instruction>) -> Result<BiteCodeEvaluated, String> {
    let mut vm = VM::new();
    vm.run(&code)?;
    return Ok(vm.result().unwrap_or(&BiteCodeEvaluated::Int(0)).clone());
}

use std::str::Chars;

#[derive(Debug)]
struct P<'a> {
    s: &'a str,
    chars: Chars<'a>,
    i: usize, // byte index
}

impl<'a> P<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s,
            chars: s.chars(),
            i: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        let ch_len = ch.len_utf8();
        self.i += ch_len;
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }

    fn expect_str(&mut self, expected: &str) -> Result<(), String> {
        self.skip_ws();
        if self.s[self.i..].starts_with(expected) {
            // advance
            for _ in 0..expected.len() {
                self.next_char();
            }
            Ok(())
        } else {
            Err(format!("Error! Expected `{}` at pos {}", expected, self.i))
        }
    }

    fn consume_if(&mut self, pat: &str) -> bool {
        self.skip_ws();
        if self.s[self.i..].starts_with(pat) {
            for _ in 0..pat.len() {
                self.next_char();
            }
            true
        } else {
            false
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.peek() != Some('"') {
            return Err(format!("Error! Expected string at pos {}", self.i));
        }
        // consume opening "
        self.next_char();
        let mut out = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '"' => return Ok(out),
                '\\' => {
                    if let Some(esc) = self.next_char() {
                        match esc {
                            '"' => out.push('"'),
                            '\\' => out.push('\\'),
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            other => out.push(other),
                        }
                    } else {
                        return Err("Error! Unterminated escape".to_string());
                    }
                }
                c => out.push(c),
            }
        }
        Err("Error! Unterminated string literal".to_string())
    }

    fn parse_number(&mut self) -> Result<i32, String> {
        self.skip_ws();
        let mut start = self.i;
        if self.peek() == Some('-') {
            self.next_char();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.next_char();
            } else {
                break;
            }
        }
        if start == self.i {
            return Err(format!("Error! Expected number at {}", start));
        }
        let slice = &self.s[start..self.i];
        slice
            .parse::<i32>()
            .map_err(|e| format!("Invalid number `{}`: {}", slice, e))
    }

    fn parse_bool(&mut self) -> Result<bool, String> {
        self.skip_ws();
        let name = self.parse_ident();
        name.parse::<bool>()
            .map_err(|e| format!("Error Invalid boolean `{}`: {}", name, e))
    }

    fn parse_ident(&mut self) -> String {
        self.skip_ws();
        let mut out = String::new();
        while let Some(c) = self.peek() {
            // identifier chars: ASCII letters,digits,underscore,punctuation used in names like ':' '-' '?',etc.
            // stop at delimiters: whitespace,',',']',')'
            if c.is_whitespace() || c == ',' || c == ']' || c == ')' {
                break;
            }
            // also stop when encountering '(' because ident followed by '(' is function-like name
            if c == '(' || c == '[' || c == ']' || c == '{' || c == ')' {
                break;
            }
            out.push(c);
            self.next_char();
        }
        out
    }

    // parse vec![ ... ] that contains instruction-like elements
    fn parse_vec_instructions(&mut self) -> Result<Vec<Instruction>, String> {
        self.skip_ws();
        // expect "vec!"
        // self.expect_str("vec!")?;
        // self.skip_ws();
        self.expect_str("[")?;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.consume_if("]") {
                break;
            }
            let instr = self.parse_instruction()?;
            items.push(instr);
            self.skip_ws();
            if self.consume_if(",") {
                continue;
            } else if self.consume_if("]") {
                break;
            } else {
                return Err(format!(
                    "Error! Expected `,` or `]` after item at pos {}",
                    self.i
                ));
            }
        }
        Ok(items)
    }

    // parse vec!["x","y"] => Vec<String>
    fn parse_vec_strings(&mut self) -> Result<Vec<String>, String> {
        self.skip_ws();
        // self.expect_str("vec!")?;
        // self.skip_ws();
        self.expect_str("[")?;
        let mut out = Vec::new();
        loop {
            self.skip_ws();
            if self.consume_if("]") {
                break;
            }
            let s = self.parse_string()?;
            out.push(s);
            self.skip_ws();
            if self.consume_if(",") {
                continue;
            } else if self.consume_if("]") {
                break;
            } else {
                return Err(format!(
                    "Expected `,` or `]` in vec![strings] at pos {}",
                    self.i
                ));
            }
        }
        Ok(out)
    }

    // parse a single instruction; it recognizes named instructions and their args
    fn parse_instruction(&mut self) -> Result<Instruction, String> {
        self.skip_ws();
        // Many elements are function-like: Name(...) or bare Name
        // Read identifier (could be MakeLambda,StoreVar,PushInt,Pop,etc.)
        let name = self.parse_ident();
        if name.is_empty() {
            return Err(format!("Error! Expected instruction at pos {}", self.i));
        }

        // If next non-ws char is '(' then there are args,else it's a bare instruction name
        self.skip_ws();
        if self.peek() == Some('(') {
            self.next_char(); // consume '('
                              // parse according to name
            let res = match name.as_str() {
                "MakeLambda" => {
                    // ( vec!["x",...] ,vec![ instrs ... ] )
                    self.skip_ws();
                    let params = self.parse_vec_strings()?;
                    self.skip_ws();
                    self.expect_str(",")?;
                    self.skip_ws();
                    let body = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::MakeLambda(params, body))
                }
                "StoreVar" => {
                    // ( "some:name" )
                    self.skip_ws();
                    let s = self.parse_string()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::StoreVar(s))
                }
                "PushInt" => {
                    self.skip_ws();
                    let n = self.parse_number()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::PushInt(n))
                }
                "PushBool" => {
                    self.skip_ws();
                    let n = self.parse_bool()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::PushBool(n))
                }
                "Call" => {
                    self.skip_ws();
                    let n = self.parse_number()? as usize;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::Call(n))
                }
                "MakeVector" => {
                    self.skip_ws();
                    let n = self.parse_number()? as usize;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::MakeVector(n))
                }
                "If" => {
                    // ( vec![then_instrs],vec![else_instrs] )
                    self.skip_ws();
                    let then_branch = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(",")?;
                    self.skip_ws();
                    let else_branch = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::If(then_branch, else_branch))
                }
                "Loop" => {
                    // ( vec![start],vec![end],vec![func] )
                    self.skip_ws();
                    let start = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(",")?;
                    self.skip_ws();
                    let end = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(",")?;
                    self.skip_ws();
                    let func = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::Loop(start, end, func))
                }
                "LoopFinish" => {
                    self.skip_ws();
                    let cond = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(",")?;
                    self.skip_ws();
                    let func = self.parse_vec_instructions()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::LoopFinish(cond, func))
                }
                // Some instructions take string arg (LoadVar) or number etc.
                "LoadVar" => {
                    self.skip_ws();
                    let s = self.parse_string()?;
                    self.skip_ws();
                    self.expect_str(")")?;
                    Ok(Instruction::LoadVar(s))
                }
                other => Err(format!(
                    "Unknown instruction with args: {} at pos {}",
                    other, self.i
                )),
            };
            return res;
        } else {
            // bare name: Pop,Length,GetArray,SetArray,Eq,Add,Mult,Div,Sub,Mod,LoadVar ops without args,etc.
            let instr = self.parse_instruction_named(&name)?;
            return Ok(instr);
        }
    }

    fn parse_instruction_named(&self, name: &str) -> Result<Instruction, String> {
        match name {
            "PushInt" => Err("Error! PushInt requires an argument".into()),
            "PushBool" => Err("Error! PushBool requires an argument".into()),
            // LoadVar without parens isn't used in your data,but map to error to be explicit
            // You could accept bare identifiers and try to map them to LoadVar with that name,if desired.
            "MakeLambda" => Err("Error! MakeLambda expects args".into()),
            // If you have bare names that are synonyms for zero-arg instructions,add here:
            "LoadVar" => Err("Error! LoadVar expects a string arg".into()),

            "Pop" => Ok(Instruction::Pop),
            "Length" => Ok(Instruction::Length),
            "GetArray" => Ok(Instruction::GetArray),
            "SetArray" => Ok(Instruction::SetArray),
            "PopArray" => Ok(Instruction::PopArray),
            "Eq" => Ok(Instruction::Eq),
            "EqBool" => Ok(Instruction::EqBool),
            "Add" => Ok(Instruction::Add),
            "Mult" => Ok(Instruction::Mult),
            "Div" => Ok(Instruction::Div),
            "Sub" => Ok(Instruction::Sub),
            "Mod" => Ok(Instruction::Mod),

            // also catch `LoadVar("...")` handled above
            "Pop," => Ok(Instruction::Pop), // tolerance for trailing commas
            "Lt" => Ok(Instruction::Lt),
            "Gt" => Ok(Instruction::Gt),
            "Gte" => Ok(Instruction::Gte),
            "Lte" => Ok(Instruction::Lte),
            "Not" => Ok(Instruction::Not),

            "BitXor" => Ok(Instruction::BitXor),
            "BitRs" => Ok(Instruction::BitRs),
            "BitLs" => Ok(Instruction::BitLs),
            "BitNot" => Ok(Instruction::BitNot),
            "BitOr" => Ok(Instruction::BitOr),
            "BitAnd" => Ok(Instruction::BitAnd),

            other => Err(format!("Error! Unknown bare instruction `{}`", other)),
        }
    }
}

/// Public parser entrypoint
pub fn parse_bitecode(s: &str) -> Result<Vec<Instruction>, String> {
    let mut p = P::new(s);
    p.skip_ws();
    // Expect top-level vec![ ... ]
    let v = p.parse_vec_instructions()?;
    p.skip_ws();
    if p.i < s.len() {
        // allow trailing whitespace only
        if s[p.i..].trim().is_empty() {
            Ok(v)
        } else {
            Err(format!(
                "Error! Trailing data after top-level vec! at pos {}",
                p.i
            ))
        }
    } else {
        Ok(v)
    }
}
