use crate::codegen::{body, generate_instr, generate_value, spamtest, BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;
use sieve::compiler::grammar::instruction::Instruction;

pub(crate) fn generate_test(
    ctx: &mut CodeGen,
    node: &sieve_grammar::test::Test,
) -> Result<(), BoxError> {
    let jz = match ctx.eat() {
        Instruction::Jz(jz) => *jz,
        e => unreachable!("invalid Jump instruction: {e:?}"),
    };

    ctx.buffer.write("if (");

    match node {
        sieve_grammar::test::Test::Address(addr) => {
            if addr.header_list.len() != 1 {
                return Err(
                    format!("unsupported header_list len: {}", addr.header_list.len()).into(),
                );
            }
            if addr.key_list.len() != 1 {
                return Err(format!("unsupported key_list len: {}", addr.key_list.len()).into());
            }
            if addr.match_type != sieve_grammar::MatchType::Is {
                return Err(format!("unsupported match_type: {:?}", addr.match_type).into());
            }

            let header = addr.header_list.first().ok_or("expect one element")?;

            match header {
                sieve::compiler::Value::Text(s) => {
                    if s.to_lowercase() != "to" {
                        return Err(format!("unsupported header: {s}").into());
                    }

                    ctx.buffer.write("parsedMessage.to[0].address");
                }

                e => return Err(format!("address test for header not implemented: {e:?}").into()),
            }

            ctx.buffer.write("===");

            for item in &addr.key_list {
                generate_value(ctx, &item)?;
            }
        }

        sieve_grammar::test::Test::Header(node) => {
            if node.header_list.len() != 1 {
                return Err(
                    format!("unsupported header_list len: {}", node.header_list.len()).into(),
                );
            }
            if node.key_list.len() != 1 {
                return Err(format!("unsupported key_list len: {}", node.key_list.len()).into());
            }

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

                sieve_grammar::MatchType::Matches(_) => {
                    ctx.buffer.write(".match(");

                    let key = node.key_list.first().ok_or("expect one element")?;
                    if let sieve::compiler::Value::Text(s) = key {
                        ctx.buffer.write("/");
                        ctx.buffer.write(&sieve_to_js_regex(&s));
                        ctx.buffer.write("/");
                    };

                    ctx.buffer.write(")");
                }

                e => todo!("match type: {e:?}"),
            }
        }

        sieve_grammar::test::Test::Vacation(_node) => {
            // FIXME: for now there's no test for Vacation, we just execute the
            // rule.
            ctx.buffer.write("true");
        }

        sieve_grammar::test::Test::String(node) => {
            if node.match_type != sieve_grammar::MatchType::Is {
                return Err(format!("unsupported match_type: {:?}", node.match_type).into());
            }
            if node.comparator != sieve_grammar::Comparator::AsciiCaseMap {
                return Err(format!("unsupported comparator: {:?}", node.comparator).into());
            }

            let source = node.source.first().ok_or("expect one element")?;
            generate_value(ctx, source)?;

            ctx.buffer.write("===");

            let key = node.key_list.first().ok_or("expect one element")?;
            generate_value(ctx, key)?;
        }

        sieve_grammar::test::Test::Body(n) => {
            body::generate_test_body(ctx, n)?;
        }

        sieve_grammar::test::Test::SpamTest(n) => {
            spamtest::generate_test_spamtest(ctx, n)?;
        }

        e => todo!("test not implemented {:?}", e),
    };

    ctx.buffer.write(") {");

    while ctx.cursor < jz {
        let instr = ctx.eat();
        generate_instr(ctx, instr)?;
    }

    ctx.buffer.write("}");

    Ok(())
}

fn sieve_to_js_regex(v: &str) -> String {
    v.replace("*", ".*")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::GenerateOpts;
    use std::sync::Arc;

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
            "if (parsedMessage.to[0].address===\"match\") {return;return;}"
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
            "if (parsedMessage.headers[\"x-header\"].value.includes(\"match\")) {return;return;}"
        );
    }

    #[test]
    fn test_generate_test_header_match() {
        let test =
            sieve_grammar::test::Test::Header(sieve_grammar::tests::test_header::TestHeader {
                header_list: vec![sieve::compiler::Value::Text(Arc::new("subject".to_owned()))],
                key_list: vec![sieve::compiler::Value::Text(Arc::new("*".to_owned()))],
                match_type: sieve_grammar::MatchType::Matches(2),
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
            "if (parsedMessage.subject.match(/.*/)) {return;return;}"
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
            "if (parsedMessage.subject.includes(\"match\")) {return;return;}"
        );
    }

    #[test]
    fn test_generate_test_string() {
        let test =
            sieve_grammar::test::Test::String(sieve_grammar::tests::test_string::TestString {
                match_type: sieve_grammar::MatchType::Is,
                comparator: sieve_grammar::Comparator::AsciiCaseMap,
                source: vec![sieve::compiler::Value::Text(Arc::new("test".to_owned()))],
                key_list: vec![sieve::compiler::Value::Text(Arc::new("Y".to_owned()))],
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
            "if (\"test\"===\"Y\") {return;return;}"
        );
    }
}
