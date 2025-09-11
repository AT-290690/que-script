use crate::parser::Expression::*;
macro_rules! s {($s:expr) => { $s.to_string() }}
pub fn load_ast() -> crate::parser::Expression {
Apply(vec![Word(s!("do")), Apply(vec![Word(s!("+")), Atom(1), Atom(2)])])
}
