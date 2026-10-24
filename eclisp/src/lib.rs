pub mod externals;
pub mod infer;
pub mod parser;
pub mod types;

pub use infer::{EffectFlags, InferErrorInfo, TypedExpression};
pub use parser::{DecimalLiteral, Expression};
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

    #[test]
    fn collects_independent_top_level_type_errors_without_cascades() {
        let source = r#"
            (let bad (+ true 1))
            (let uses-bad (+ bad 1))
            (let also-bad (missing-name 2))
            (let good (+ 20 22))
        "#;
        let expr = build(source).expect("program should parse");
        let errors = infer::infer_with_builtins_typed_lsp_all(&expr, core_environment(), 4)
            .expect_err("program should have recoverable type errors");

        assert_eq!(errors.len(), 2, "unexpected diagnostics: {errors:#?}");
        assert!(
            errors[0].message.contains("expected Int but got Bool"),
            "unexpected first diagnostic: {:?}",
            errors[0]
        );
        assert!(
            errors[1].message.contains("Undefined variable: missing-name"),
            "unexpected second diagnostic: {:?}",
            errors[1]
        );
        assert_eq!(errors[0].scope.as_ref().map(|scope| scope.user_top_form), Some(0));
        assert_eq!(errors[1].scope.as_ref().map(|scope| scope.user_top_form), Some(2));
    }

    #[test]
    fn decimal_literals_remain_exact_until_runtime_quantization() {
        let values =
            parse("1000000.001 1000000.002 3.1425 -3.1435").expect("decimal literals should parse");
        let rendered = values.iter().map(Expression::to_lisp).collect::<Vec<_>>();
        assert_eq!(
            rendered,
            ["1000000.001", "1000000.002", "3.1425", "-3.1435"]
        );

        let scaled = values
            .iter()
            .map(|value| match value {
                Expression::Dec(value) => value.scaled_i64(1000).unwrap(),
                _ => panic!("expected Dec literal"),
            })
            .collect::<Vec<_>>();
        assert_eq!(scaled, [1_000_000_001, 1_000_000_002, 3_142, -3_144]);
    }
}
