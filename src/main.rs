mod lisp;
use std::fs;
use std::path::Path;

fn main() {
    let path = Path::new("./lisp/main.lisp");
    let file = fs::read_to_string(path).expect("Unable to read main.lisp");
    lisp::eval(&file);
}
