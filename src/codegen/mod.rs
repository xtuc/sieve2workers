use crate::BoxError;
use sieve::compiler::grammar as sieve_grammar;
use sieve::compiler::grammar::instruction::Instruction;

mod buffer;

#[derive(Default)]
pub(crate) struct GenerateOpts {
    pub(crate) debug: bool,
}

pub struct CodeGen<'a> {
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
            .write("export async function run({ message, sendMessage }) {");
        self.buffer.newline();

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
        // self.buffer.write("await message.forward('default here');");

        self.buffer.newline();
        self.buffer.write("}");

        Ok(self.buffer.to_string())
    }
}

fn generate_instr(ctx: &mut CodeGen, instr: &Instruction) -> Result<(), BoxError> {
    match instr {
        Instruction::Test(n) => {
            // Generate a Test instruction and its content
            // For isolation between rules, each test are wrapped into try/catch.
            // That prevents one rule of crashing the entire email routing.
            ctx.buffer.newline();
            ctx.buffer.write("try {");

            generate_test(ctx, &n)?;

            ctx.buffer.newline();
            ctx.buffer.write("} catch (err) {");
            ctx.buffer.newline();

            let rule_id = ctx.cursor;
            ctx.buffer.write(&format!(
                "console.log('rule {rule_id} failed and has been skipped', err);"
            ));
            ctx.buffer.write("}");
        }
        Instruction::Reject(n) => generate_reject(ctx, &n)?,
        Instruction::Redirect(n) => generate_redirect(ctx, &n)?,
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
        Instruction::Require(_) => {}
        e => todo!("{:?}", e),
    }

    Ok(())
}

fn generate_test(ctx: &mut CodeGen, node: &sieve_grammar::test::Test) -> Result<(), BoxError> {
    let jz = match ctx.eat() {
        Instruction::Jz(jz) => *jz,
        e => unreachable!("invalid Jump instruction: {e:?}"),
    };
    let consequent = &ctx.instructions[ctx.cursor..jz];
    ctx.cursor = jz;

    match node {
        sieve_grammar::test::Test::Address(addr) => {
            ctx.buffer.write("if (");

            assert_eq!(addr.header_list.len(), 1);
            assert_eq!(addr.key_list.len(), 1);
            assert_eq!(addr.match_type, sieve_grammar::MatchType::Is);

            let header = addr.header_list.first().ok_or("expect one element")?;

            match header {
                sieve::compiler::Value::Text(s) => {
                    let s = s.to_lowercase();
                    assert_eq!(s, "to");

                    ctx.buffer.write("parsedMessage.to[0].address");
                }

                e => unimplemented!("address test for header {e:?}"),
            }

            ctx.buffer.write("===");

            for item in &addr.key_list {
                generate_value(ctx, &item)?;
            }

            ctx.buffer.write(") {");
            ctx.buffer.newline();

            for instr in consequent {
                generate_instr(ctx, instr)?;
            }
            ctx.buffer.newline();

            ctx.buffer.write("}");
            ctx.buffer.newline();
        }

        sieve_grammar::test::Test::Header(node) => {
            ctx.buffer.write("if (");

            assert_eq!(node.header_list.len(), 1);
            assert_eq!(node.key_list.len(), 1);

            let header = node.header_list.first().ok_or("expect one element")?;

            match header {
                sieve::compiler::Value::Text(v) => match &*v.to_lowercase() {
                    "subject" => {
                        ctx.buffer.write("parsedMessage.subject");
                    }

                    _ => {
                        ctx.buffer.write("parsedMessage.headers[");
                        generate_value(ctx, &header)?;
                        ctx.buffer.write("].value");
                    }
                },

                _ => {
                    ctx.buffer.write("parsedMessage.headers[");
                    generate_value(ctx, &header)?;
                    ctx.buffer.write("].value");
                }
            }

            match node.match_type {
                sieve_grammar::MatchType::Is => {
                    ctx.buffer.write("===");

                    for item in &node.key_list {
                        generate_value(ctx, &item)?;
                    }
                }

                sieve_grammar::MatchType::Contains => {
                    ctx.buffer.write(".includes(");

                    for item in &node.key_list {
                        generate_value(ctx, &item)?;
                    }

                    ctx.buffer.write(")");
                }

                e => todo!("match type: {e:?}"),
            }

            ctx.buffer.write(") {");
            ctx.buffer.newline();

            for instr in consequent {
                generate_instr(ctx, instr)?;
            }
            ctx.buffer.newline();

            ctx.buffer.write("}");
            ctx.buffer.newline();
        }

        e => todo!("test not implemented {:?}", e),
    }

    Ok(())
}

fn generate_reject(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_reject::Reject,
) -> Result<(), BoxError> {
    ctx.buffer.write("message.setReject(");
    generate_value(ctx, &node.reason)?;
    ctx.buffer.write(");");
    Ok(())
}

fn generate_redirect(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_redirect::Redirect,
) -> Result<(), BoxError> {
    if ctx.opts.debug {
        ctx.buffer.write("console.log(\"forward\");");
    }

    ctx.buffer.write("message.forward(");
    generate_value(ctx, &node.address)?;
    ctx.buffer.write(");");
    Ok(())
}

fn generate_value(ctx: &mut CodeGen, node: &sieve::compiler::Value) -> Result<(), BoxError> {
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
    fn test_generate_reject() {
        let mut ctx = CodeGen::new(GenerateOpts::default(), &[]);
        let input = sieve_grammar::actions::action_reject::Reject {
            ereject: false,
            reason: sieve::compiler::Value::Text(Arc::new("foo reason".to_owned())),
        };

        generate_reject(&mut ctx, &input).unwrap();
        assert_eq!(ctx.buffer.to_string(), "message.setReject(\"foo reason\");");
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
        assert_eq!(ctx.buffer.to_string(), "message.forward(\"foo reason\");");
    }

    #[test]
    fn test_generate_test_address() {
        let test =
            sieve_grammar::test::Test::Address(sieve_grammar::tests::test_address::TestAddress {
                header_list: vec![sieve::compiler::Value::Text(Arc::new("To".to_owned()))],
                key_list: vec![sieve::compiler::Value::Text(Arc::new("match".to_owned()))],
                address_part: sieve_grammar::AddressPart::All,
                match_type: sieve_grammar::MatchType::Is,
                comparator: sieve_grammar::Comparator::AsciiCaseMap,
                index: None,
                mime_anychild: false,
                is_not: false,
            });
        let nodes = vec![
            Instruction::Jz(3),
            // consequent
            Instruction::Stop,
            Instruction::Stop,
            // continuation
            Instruction::Discard,
        ];
        let mut ctx = CodeGen::new(GenerateOpts::default(), &nodes);

        generate_test(&mut ctx, &test).unwrap();
        assert_eq!(
            ctx.buffer.to_string(),
            "if (parsedMessage.to[0].address===\"match\") {\nreturn;return;\n}\n"
        );
    }

    #[test]
    fn test_generate_test_header_contains() {
        let test =
            sieve_grammar::test::Test::Header(sieve_grammar::tests::test_header::TestHeader {
                header_list: vec![sieve::compiler::Value::Text(Arc::new(
                    "x-header".to_owned(),
                ))],
                key_list: vec![sieve::compiler::Value::Text(Arc::new("match".to_owned()))],
                match_type: sieve_grammar::MatchType::Contains,
                comparator: sieve_grammar::Comparator::AsciiCaseMap,
                mime_opts: sieve_grammar::actions::action_mime::MimeOpts::None,
                index: None,
                mime_anychild: false,
                is_not: false,
            });
        let nodes = vec![
            Instruction::Jz(3),
            // consequent
            Instruction::Stop,
            Instruction::Stop,
            // continuation
            Instruction::Discard,
        ];
        let mut ctx = CodeGen::new(GenerateOpts::default(), &nodes);

        generate_test(&mut ctx, &test).unwrap();
        assert_eq!(
            ctx.buffer.to_string(),
            "if (parsedMessage.headers[\"x-header\"].value.includes(\"match\")) {\nreturn;return;\n}\n"
        );
    }

    #[test]
    fn test_generate_test_header_contains_well_known_header() {
        let test =
            sieve_grammar::test::Test::Header(sieve_grammar::tests::test_header::TestHeader {
                header_list: vec![sieve::compiler::Value::Text(Arc::new("SuBJecT".to_owned()))],
                key_list: vec![sieve::compiler::Value::Text(Arc::new("match".to_owned()))],
                match_type: sieve_grammar::MatchType::Contains,
                comparator: sieve_grammar::Comparator::AsciiCaseMap,
                mime_opts: sieve_grammar::actions::action_mime::MimeOpts::None,
                index: None,
                mime_anychild: false,
                is_not: false,
            });
        let nodes = vec![
            Instruction::Jz(3),
            // consequent
            Instruction::Stop,
            Instruction::Stop,
            // continuation
            Instruction::Discard,
        ];
        let mut ctx = CodeGen::new(GenerateOpts::default(), &nodes);

        generate_test(&mut ctx, &test).unwrap();
        assert_eq!(
            ctx.buffer.to_string(),
            "if (parsedMessage.subject.includes(\"match\")) {\nreturn;return;\n}\n"
        );
    }
}
