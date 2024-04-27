use crate::codegen::{BoxError, CodeGen};
use sieve::compiler::grammar as sieve_grammar;

pub(crate) fn generate_vacation(
    ctx: &mut CodeGen,
    node: &sieve_grammar::actions::action_vacation::Vacation,
) -> Result<(), BoxError> {
    let data = if let sieve::compiler::Value::Text(s) = &node.reason {
        s
    } else {
        unimplemented!();
    };

    let subject = if let Some(sieve::compiler::Value::Text(s)) = &node.subject {
        s
    } else {
        "Vacation auto-reply"
    };

    let from = ctx
        .opts
        .vacation_from_address
        .as_ref()
        .ok_or("Missing --vacation-from-address")?;

    ctx.buffer.write(&format!(
        r#"
        const msg = createMimeMessage();
        msg.setHeader("In-Reply-To", message.headers.get("Message-ID"));
        msg.setSender({{ name: "Vacation auto-reply", addr: "{from}" }});
        msg.setRecipient(message.from);
        msg.setSubject("{subject}");
        msg.addMessage({{
          contentType: 'text/plain',
          data: "{data}"
        }});

        const replyMessage = new EmailMessage(
          "{from}",
          message.from,
          msg.asRaw()
        );

        await message.reply(replyMessage);
        "#,
    ));

    Ok(())
}
