use crate::lisp::Expression;
pub fn load_ast() -> Expression {
    Expression::Apply(vec![Expression::Word("do".to_string()), Expression::Apply(vec![Expression::Word("let".to_string()), Expression::Word("process".to_string()), Expression::Apply(vec![Expression::Word("lambda".to_string()), Expression::Word("xs".to_string()), Expression::Apply(vec![Expression::Word("get".to_string()), Expression::Word("xs".to_string()), Expression::Atom(0)])])]), Expression::Apply(vec![Expression::Word("process".to_string()), Expression::Apply(vec![Expression::Word("array".to_string()), Expression::Atom(1), Expression::Atom(2), Expression::Atom(3)])])])
}
