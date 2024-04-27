use crate::codegen::{generate_value, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_test_body(
    ctx: &mut CodeGen,
    node: &sieve_grammar::tests::test_body::TestBody,
) -> Result<(), BoxError> {
    assert_eq!(
        node.body_transform,
        sieve_grammar::tests::test_body::BodyTransform::Text
    );

    ctx.buffer.write("raw.includes(");

    let key = node.key_list.first().ok_or("expect one element")?;
    generate_value(ctx, key)?;

    ctx.buffer.write(")");

    Ok(())
}
