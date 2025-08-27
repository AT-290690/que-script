
#![allow(dead_code)]
#![allow(warnings)]

use std::rc::Rc;
use std::fmt;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(value) => write!(f, "{}", value),
            Value::Function(_) => write!(f, "Function"),
            Value::Array(arr) => {
                let arr_ref = arr;
                let elements: Vec<String> = arr_ref.iter().map(|x| format!("{:?}", x)).collect();
                write!(f, "[{}]", elements.join(" "))
            }
        }
    }
}
#[derive(Clone)]
pub enum Value {
    Number(i32),
    Array(Vec<Value>),
    Function(Rc<dyn Fn(Vec<Value>) -> Value>),
}
    
// PartialEq for comparing Value with Value
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            _ => false,
        }
    }
}

// PartialEq for comparing Value with i32
impl PartialEq<i32> for Value {
    fn eq(&self, other: &i32) -> bool {
        match self {
            Value::Number(a) => a == other,
            _ => false,
        }
    }
}

pub fn _add(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x + y),
        _ => panic!("+ expects two numbers, got non-number Value"),
    }
}

pub fn _sub(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x - y),
        _ => panic!("- expects two numbers, got non-number Value"),
    }
}

pub fn _mult(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x * y),
        _ => panic!("* expects two numbers, got non-number Value"),
    }
}

pub fn _div(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x / y),
        _ => panic!("/ expects two numbers, got non-number Value"),
    }
}

pub fn _eq(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if x == y { 1 } else { 0 }),
        _ => panic!("= expects two numbers, got non-number Value"),
    }
}

pub fn _gt(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if x > y { 1 } else { 0 }),
        _ => panic!("> expects two numbers, got non-number Value"),
    }
}

pub fn _lt(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if x < y { 1 } else { 0 }),
        _ => panic!("< expects two numbers, got non-number Value"),
    }
}


pub fn _lte(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if x <= y { 1 } else { 0 }),
        _ => panic!("<= expects two numbers, got non-number Value"),
    }
}

pub fn _gte(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if x >= y { 1 } else { 0 }),
        _ => panic!(">= expects two numbers, got non-number Value"),
    }
}

pub fn or(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if (*x == 1 || *y == 1) { 1 } else { 0 }),
        _ => panic!("or expects two numbers, got non-number Value"),
    }
}

pub fn and(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(if (*x == 1 && *y == 1)  { 1 } else { 0 }),
        _ => panic!("and expects two numbers, got non-number Value"),
    }
}

pub fn _mod(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x % y ),
        _ => panic!("mod expects two numbers, got non-number Value"),
    }
}

pub fn _bit_and(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x & y ),
        _ => panic!("& expects two numbers, got non-number Value"),
    }
}

pub fn _bit_or(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x | y ),
        _ => panic!("| expects two numbers, got non-number Value"),
    }
}

pub fn _bit_not(a: &Value) -> Value {
    match (a) {
        Value::Number(x) => Value::Number(!x),
        _ => panic!("~ expects two numbers, got non-number Value"),
    }
}

pub fn _bit_xor(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x ^ y ),
        _ => panic!("^ expects two numbers, got non-number Value"),
    }
}

pub fn _bit_right_shift(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x >> y ),
        _ => panic!(">> expects two numbers, got non-number Value"),
    }
}

pub fn _bit_left_shift(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x << y ),
        _ => panic!("<< expects two numbers, got non-number Value"),
    }
}

pub fn not(a: &Value) -> Value {
    match (a) {
        Value::Number(x) => Value::Number(if *x == 0 { 1 } else { 0 }),
        _ => panic!("not expects two numbers, got non-number Value"),
    }
}

pub fn _set_mutate(array: &mut Value, index: &Value, value: &Value) -> Value {
    match array {
        Value::Array(ref mut arr) => {
            let idx = match index {
                Value::Number(n) if *n >= 0 => *n as usize,
                _ => panic!("Index must be a non-negative number"),
            };

            if idx < arr.len() {
                arr[idx] = value.clone(); // set existing element
            } else if idx == arr.len() {
                arr.push(value.clone()); // append at the end
            } else {
                panic!("Index out of bounds");
            }
             return Value::Number(0)
        }
        _ => panic!("First argument to set! must be an array"),
    }
}
pub fn _pop_mutate(array: &mut Value) -> Value {
    match array {
        Value::Array(ref mut arr) => {
             arr.pop();
            return Value::Number(0)
        }
        _ => panic!("First argument to pop! must be an array"),
    }
}

pub fn get(array: &Value, index: &Value) -> Value {
    let arr = match array {
        Value::Array(a) => a,
        _ => panic!("First argument to get must be an array"),
    };

    let idx = match index {
        Value::Number(n) => *n as usize,
        _ => panic!("Second argument to get must be a number"),
    };

    arr[idx].clone() // return a clone of the element
}
pub fn length(array: &Value) -> Value {
       match (array) {
        Value::Array(x) => Value::Number(x.len() as i32),
        _ => panic!("First argument to length must be an array"),
    }
}
pub fn lambda<F>(f: F) -> Value
where
    F: 'static + Fn(Vec<&Value>) -> Value,
{
    Value::Function(Rc::new(move |args: Vec<Value>| {
        // Convert Vec<Value> to Vec<&Value> for the inner closure
        let refs: Vec<&Value> = args.iter().collect();
        f(refs)
    }))
}
 fn call(func: &Value, args: Vec<Value>) -> Value {
    if let Value::Function(f) = func {
        f(args)
    } else {
        panic!("Not a function");
    }
 }   

pub fn run () -> Value {
    let mut xs_mutate: Value = Value::Array(vec![Value::Number(1), Value::Number(2), Value::Number(3)]);;
_set_mutate(&mut xs_mutate, &Value::Number(0), &Value::Number(10));
_set_mutate(&mut xs_mutate, &Value::Number(1), &Value::Number(10));
_set_mutate(&mut xs_mutate, &Value::Number(2), &Value::Number(30));
_set_mutate(&mut xs_mutate, &Value::Number(3), &Value::Number(30));
_set_mutate(&mut xs_mutate, &Value::Number(4), &Value::Number(30));
xs_mutate}
pub fn main () {
println!("{:?}",run());}
