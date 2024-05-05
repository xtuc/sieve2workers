use crate::codegen::js::{generate_value, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_add_header(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_editheader::AddHeader,
) -> Result<(), BoxError> {
    match &node.field_name {
        sieve::compiler::Value::Text(s) => {
            if !s.to_lowercase().starts_with("x-") {
                return Err(format!("header {} not allowed", s).into());
            }

            ctx.buffer.write("extraHeaders.append(");
            generate_value(ctx, &node.field_name)?;
            ctx.buffer.write(",");
            generate_value(ctx, &node.value)?;
            ctx.buffer.write(");");
        }

        e => return Err(format!("add header field not implemented: {e:?}").into()),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::GenerateOpts;
    use std::sync::Arc;

    #[test]
    fn test_generate_add_header() {
        let mut ctx = CodeGen::new(GenerateOpts::default(), &[]);

        {
            let node = sieve_grammar::actions::action_editheader::AddHeader {
                field_name: sieve::compiler::Value::Text(Arc::new("invalid_header".to_owned())),
                value: sieve::compiler::Value::Text(Arc::new("b".to_owned())),
                last: false,
            };

            generate_add_header(&mut ctx, &node).unwrap_err();
        }

        {
            let node = sieve_grammar::actions::action_editheader::AddHeader {
                field_name: sieve::compiler::Value::Text(Arc::new("x-a".to_owned())),
                value: sieve::compiler::Value::Text(Arc::new("b".to_owned())),
                last: false,
            };

            generate_add_header(&mut ctx, &node).unwrap();
            assert_eq!(
                ctx.buffer.to_string(),
                "extraHeaders.append(\"x-a\",\"b\");"
            );
        }
    }
}
