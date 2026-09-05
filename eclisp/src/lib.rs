pub mod externals;
pub mod infer;
pub mod parser;
pub mod types;

pub use infer::{EffectFlags, InferErrorInfo, TypedExpression};
pub use parser::Expression;
pub use types::{Type, TypeEnv, TypeScheme, TypeVar};

pub fn parse(source: &str) -> Result<Vec<Expression>, String> {
    parser::parse(source)
}

pub fn build(source: &str) -> Result<Expression, String> {
    parser::build(source)
}

pub fn core_environment() -> (TypeEnv, u64) {
    types::create_builtin_environment(TypeEnv::new())
}

pub fn infer(expr: &Expression) -> Result<(Type, TypedExpression), String> {
    infer::infer_with_builtins_typed(expr, core_environment())
}

pub fn check(source: &str) -> Result<(Type, TypedExpression), String> {
    let expr = build(source)?;
    infer(&expr)
}

pub fn check_with_error_info(source: &str) -> Result<(Type, TypedExpression), InferErrorInfo> {
    let expr = build(source).map_err(|message| InferErrorInfo {
        message,
        scope: None,
        snippet: None,
        partial_typed_ast: None,
    })?;
    infer::infer_with_builtins_typed_lsp(&expr, core_environment(), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_simple_program() {
        let (typ, typed) =
            check("(let add (lambda (a b) (+ a b))) (add 1 2)").expect("program should type-check");

        assert_eq!(typ, Type::Int);
        assert!(typed.effect.is_pure());
    }

    #[test]
    fn reports_type_errors() {
        let err = check("(+ 1 true)").expect_err("program should fail type-checking");

        assert!(err.contains("expected Int but got Bool"), "got: {err}");
    }
}
