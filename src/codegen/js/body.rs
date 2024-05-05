use crate::codegen::js::{generate_value, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_test_body(
    ctx: &mut CodeGen,
    node: &sieve_grammar::tests::test_body::TestBody,
) -> Result<(), BoxError> {
    if node.body_transform != sieve_grammar::tests::test_body::BodyTransform::Text {
        return Err(format!("unsupported body_transform: {:?}", node.body_transform).into());
    }

    for i in 0..node.key_list.len() {
        let key = &node.key_list[i];

        ctx.buffer.write("raw.includes(");
        generate_value(ctx, key)?;
        ctx.buffer.write(")");

        if i < node.key_list.len() - 1 {
            ctx.buffer.write(" || ");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::GenerateOpts;
    use std::sync::Arc;

    #[test]
    fn test_generate_test_body() {
        let test = sieve_grammar::tests::test_body::TestBody {
            key_list: vec![
                sieve::compiler::Value::Text(Arc::new("a".to_owned())),
                sieve::compiler::Value::Text(Arc::new("b".to_owned())),
                sieve::compiler::Value::Text(Arc::new("c".to_owned())),
            ],
            body_transform: sieve_grammar::tests::test_body::BodyTransform::Text,
            match_type: sieve_grammar::MatchType::Contains,
            comparator: sieve_grammar::Comparator::AsciiCaseMap,
            include_subject: false,
            is_not: false,
        };
        let mut ctx = CodeGen::new(GenerateOpts::default(), &[]);

        generate_test_body(&mut ctx, &test).unwrap();
        assert_eq!(
            ctx.buffer.to_string(),
            "raw.includes(\"a\") || raw.includes(\"b\") || raw.includes(\"c\")"
        );
    }
}
