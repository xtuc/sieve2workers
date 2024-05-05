use super::{buffer, GenerateOpts};
use crate::BoxError;
use sieve::compiler::grammar as sieve_grammar;
use sieve::compiler::grammar::instruction::Instruction;

mod body;
mod editheader;
mod fileinto;
mod reject;
mod relational;
mod spamtest;
mod test;
mod vacation;

pub(crate) struct CodeGen<'a> {
    instructions: &'a [Instruction],
    cursor: usize,
    buffer: buffer::Buffer,
    opts: GenerateOpts,
}

impl<'a> CodeGen<'a> {
    pub fn new(opts: GenerateOpts, instructions: &'a [Instruction]) -> Self {
        Self {
            opts,
            instructions,
            cursor: 0,
            buffer: buffer::Buffer::new(),
        }
    }

    fn eat(&mut self) -> &'a Instruction {
        let instr = &self.instructions[self.cursor];
        self.cursor += 1;

        instr
    }

    pub(crate) fn generate_js(&mut self) -> Result<String, BoxError> {
        self.buffer.write("import PostalMime from \"postal-mime\";");
        self.buffer.newline();

        self.buffer.write(
            r#"
            async function streamToArrayBuffer(stream, streamSize) {
              let result = new Uint8Array(streamSize);
              let bytesRead = 0;
              const reader = stream.getReader();
              while (true) {
                const { done, value } = await reader.read();
                if (done) {
                  break;
                }
                result.set(value, bytesRead);
                bytesRead += value.length;
              }
              return result;
            }
            "#,
        );
        self.buffer.newline();

        self.buffer
            .write("export async function run({ message, env }) {");
        self.buffer.newline();

        self.buffer.write("const extraHeaders = new Headers;");

        self.buffer
            .write("const raw = await streamToArrayBuffer(message.raw, message.rawSize);");
        self.buffer.newline();
        self.buffer
            .write("const parsedMessage = await PostalMime.parse(raw);");
        self.buffer.newline();

        if self.opts.debug {
            self.buffer
                .write("console.log('parsedMessage headers', parsedMessage.headers);");
            self.buffer
                .write("console.log('parsedMessage to', parsedMessage.to);");
            self.buffer
                .write("console.log('parsedMessage cc', parsedMessage.cc);");
            self.buffer
                .write("console.log('parsedMessage subject', parsedMessage.subject);");
            self.buffer
                .write("console.log('parsedMessage messageId', parsedMessage.messageId);");
            self.buffer
                .write("console.log('parsedMessage from', parsedMessage.from);");
        }

        while self.cursor < self.instructions.len() {
            let instr = self.eat();
            generate_instr(self, instr)?;
        }

        // TODO: sieve has a default keep rule but how to do it
        // in email workers? We don't know where to forward by default.
        // self.buffer.newline();
        // self.buffer.write("await message.forward('default here',extraHeaders);");

        self.buffer.newline();
        self.buffer.write("}");

        Ok(self.buffer.to_string())
    }
}

pub(crate) fn generate_instr(ctx: &mut CodeGen, instr: &Instruction) -> Result<(), BoxError> {
    match instr {
        Instruction::Test(n) => {
            // Generate a Test instruction and its content
            // For isolation between rules, each test are wrapped into try/catch.
            // That prevents one rule of crashing the entire email routing.
            ctx.buffer.newline();
            ctx.buffer.write("try {");

            test::generate_test(ctx, &n)?;

            ctx.buffer.newline();
            ctx.buffer.write("} catch (err) {");
            ctx.buffer.newline();

            let rule_id = ctx.cursor;
            ctx.buffer.write(&format!(
                "console.error('rule {rule_id} failed and has been skipped', err);"
            ));
            ctx.buffer.write("}");
        }
        Instruction::Reject(n) => reject::generate_reject(ctx, &n)?,
        Instruction::Redirect(n) => generate_redirect(ctx, &n)?,
        Instruction::AddHeader(n) => editheader::generate_add_header(ctx, &n)?,
        Instruction::Discard => {
            if ctx.opts.debug {
                ctx.buffer.write("console.log(\"discard\");");
            }
            ctx.buffer.write("// discard the email");
            ctx.buffer.newline();
            ctx.buffer.write("return;");
        }
        Instruction::Stop => {
            if ctx.opts.debug {
                ctx.buffer.write("console.log(\"stop\");");
            }
            ctx.buffer.write("return;");
        }
        Instruction::Keep(_) => {
            ctx.buffer.write("// keep the email");
        }
        Instruction::Require(n) => generate_require(ctx, n)?,
        Instruction::Vacation(n) => vacation::generate_vacation(ctx, &n)?,
        Instruction::Set(n) => generate_set(ctx, &n)?,
        Instruction::Clear(n) => generate_clear(ctx, &n)?,
        Instruction::FileInto(n) => fileinto::generate_fileinto(ctx, &n)?,

        e => todo!("{:?}", e),
    }

    Ok(())
}

fn generate_redirect(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_redirect::Redirect,
) -> Result<(), BoxError> {
    if ctx.opts.debug {
        ctx.buffer.write("console.log(\"forward\");");
    }

    ctx.buffer.write("await message.forward(");
    generate_value(ctx, &node.address)?;
    ctx.buffer.write(",extraHeaders);");
    Ok(())
}

fn generate_require(
    ctx: &mut CodeGen,
    capabilities: &[sieve_grammar::Capability],
) -> Result<(), BoxError> {
    for capability in capabilities {
        match capability {
            sieve_grammar::Capability::Variables => {
                ctx.buffer.write("const variables = {};");
            }

            sieve_grammar::Capability::SpamTestPlus => {
                spamtest::generate_require_spamtest(ctx)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn generate_clear(ctx: &mut CodeGen, node: &sieve_grammar::Clear) -> Result<(), BoxError> {
    if node.local_vars_idx != 0 {
        return Err(format!("unsupported local_vars_idx: {}", node.local_vars_idx).into());
    }
    if node.match_vars != 0 {
        return Err(format!("unsupported match_vars: {}", node.match_vars).into());
    }

    ctx.buffer.write("delete variables[");
    ctx.buffer
        .write_quoted(&format!("local{}", node.local_vars_num));
    ctx.buffer.write("];");

    Ok(())
}

fn generate_set(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_set::Set,
) -> Result<(), BoxError> {
    if node.modifiers.len() != 0 {
        return Err(format!("unsupported modifiers len: {}", node.modifiers.len()).into());
    }

    ctx.buffer.write("variables[");

    match &node.name {
        sieve::compiler::VariableType::Local(idx) => {
            ctx.buffer.write_quoted(&format!("local{idx}"));
        }
        e => return Err(format!("variable type not implemented: {e:?}").into()),
    }

    ctx.buffer.write("] = ");
    generate_value(ctx, &node.value)?;

    ctx.buffer.newline();
    Ok(())
}

pub(crate) fn generate_value(
    ctx: &mut CodeGen,
    node: &sieve::compiler::Value,
) -> Result<(), BoxError> {
    match node {
        sieve::compiler::Value::Text(s) => {
            ctx.buffer.write_quoted(s);
        }
        sieve::compiler::Value::Number(n) => {
            ctx.buffer.write(&n.to_string());
        }
        sieve::compiler::Value::List(list) => {
            ctx.buffer.write("[");

            for item in list {
                generate_value(ctx, item)?;
                ctx.buffer.write(",");
            }

            ctx.buffer.write("]");
        }
        e => todo!("value not implemented {:?}", e),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_generate_value() {
        let test_cases = &[
            (
                sieve::compiler::Value::Text(Arc::new("foo".to_owned())),
                r#""foo""#,
            ),
            (
                sieve::compiler::Value::Number(sieve::compiler::Number::Integer(3)),
                "3",
            ),
            (
                sieve::compiler::Value::Number(sieve::compiler::Number::Float(3.1)),
                "3.1",
            ),
            (
                sieve::compiler::Value::List(vec![
                    sieve::compiler::Value::Text(Arc::new("a".to_owned())),
                    sieve::compiler::Value::Text(Arc::new("b".to_owned())),
                ]),
                r#"["a","b",]"#,
            ),
        ];

        for (input, expected) in test_cases {
            let mut ctx = CodeGen::new(GenerateOpts::default(), &[]);
            generate_value(&mut ctx, &input).unwrap();
            assert_eq!(ctx.buffer.to_string(), expected.to_string());
        }
    }

    #[test]
    fn test_generate_redirect() {
        let mut ctx = CodeGen::new(GenerateOpts::default(), &[]);
        let input = sieve_grammar::actions::action_redirect::Redirect {
            address: sieve::compiler::Value::Text(Arc::new("foo reason".to_owned())),
            copy: false,
            notify: sieve_grammar::actions::action_redirect::Notify::Never,
            return_of_content: sieve_grammar::actions::action_redirect::Ret::Default,
            by_time: sieve_grammar::actions::action_redirect::ByTime::None,
            list: false,
        };

        generate_redirect(&mut ctx, &input).unwrap();
        assert_eq!(
            ctx.buffer.to_string(),
            "await message.forward(\"foo reason\",extraHeaders);"
        );
    }
}
