use crate::lisp::Expression;
pub fn load_ast() -> Expression {
    Expression::Apply(vec![Expression::Word("do".to_string()), Expression::Apply(vec![Expression::Word("+".to_string()), Expression::Atom(1), Expression::Atom(2)])])
}
