mod lisp;
use std::fs;
use std::path::Path;

fn main() {
    let file = fs::read_to_string(Path::new("./lisp/main.lisp")).expect("Unable to read main.lisp");
    let std = fs::read_to_string(Path::new("./lisp/std.lisp")).expect("Unable to read main.lisp");
    lisp::eval(&file, &std);
}
