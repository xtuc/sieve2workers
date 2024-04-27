use crate::codegen::{generate_value, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_reject(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_reject::Reject,
) -> Result<(), BoxError> {
    ctx.buffer.write("message.setReject(");
    generate_value(ctx, &node.reason)?;
    ctx.buffer.write(");");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::GenerateOpts;
    use std::sync::Arc;

    #[test]
    fn test_generate_reject() {
        let mut ctx = CodeGen::new(GenerateOpts::default(), &[]);
        let input = sieve_grammar::actions::action_reject::Reject {
            ereject: false,
            reason: sieve::compiler::Value::Text(Arc::new("foo reason".to_owned())),
        };

        generate_reject(&mut ctx, &input).unwrap();
        assert_eq!(ctx.buffer.to_string(), "message.setReject(\"foo reason\");");
    }
}
