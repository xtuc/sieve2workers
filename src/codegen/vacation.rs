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

    ctx.buffer.write(&format!(
        r#"
        const msg = createMimeMessage();
        msg.setHeader("In-Reply-To", message.headers.get("Message-ID"));
        msg.setSender({{ name: "Vacation auto-reply", addr: "sven@sauleau.com" }});
        msg.setRecipient(message.from);
        msg.setSubject("{subject}");
        msg.addMessage({{
          contentType: 'text/plain',
          data: "{data}"
        }});

        const replyMessage = new EmailMessage(
          "sven@sauleau.com",
          message.from,
          msg.asRaw()
        );

        await message.reply(replyMessage);
        "#,
    ));

    Ok(())
}
