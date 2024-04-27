use crate::codegen::{generate_instr, generate_value, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_require_spamtest(ctx: &mut CodeGen) -> Result<(), BoxError> {
    ctx.buffer.write(
        r#"
        async function scoreEmail() {
          const response = await env.AI.run(
            "@cf/huggingface/distilbert-sst-2-int8",
            {
              text: raw
            }
          );
        }
        "#,
    );

    Ok(())
}

pub(crate) fn generate_test_spamtest(
    ctx: &mut CodeGen,
    node: &sieve_grammar::tests::test_spamtest::TestSpamTest,
) -> Result<(), BoxError> {
    ctx.buffer.write("(await scoreEmail()) == 37");

    Ok(())
}
