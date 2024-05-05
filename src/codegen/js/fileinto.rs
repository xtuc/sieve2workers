use crate::codegen::js::{BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_fileinto(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_fileinto::FileInto,
) -> Result<(), BoxError> {
    let dest = match &node.folder {
        sieve::compiler::Value::Text(s) => s,

        e => unreachable!("invalid fileinto destination: {e:?}"),
    };

    assert!(dest.starts_with("r2://"));

    let bucket = dest.replace("r2://", "");

    ctx.buffer
        .write("const key = (parsedMessage.messageId || crypto.randomUUID()) + '.eml';");
    ctx.buffer
        .write(&format!("await env.{bucket}.put(key, raw);"));

    if ctx.opts.debug {
        ctx.buffer.write("console.log('stored email at ', key);");
    }

    Ok(())
}
