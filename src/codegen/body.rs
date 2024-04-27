use crate::codegen::{generate_instr, generate_value, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_test_body(
    ctx: &mut CodeGen,
    node: &sieve_grammar::tests::test_body::TestBody,
    jz: usize,
) -> Result<(), BoxError> {
    assert_eq!(
        node.body_transform,
        sieve_grammar::tests::test_body::BodyTransform::Text
    );

    ctx.buffer.write("if (raw.includes(");

    let key = node.key_list.first().ok_or("expect one element")?;
    generate_value(ctx, key)?;

    ctx.buffer.write(")) {");

    while ctx.cursor < jz {
        let instr = ctx.eat();
        generate_instr(ctx, instr)?;
    }

    ctx.buffer.write("}");

    Ok(())
}
