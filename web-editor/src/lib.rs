mod utils;

use wasm_bindgen::prelude::*;

const WORKER_TEMPLATE: &str = r#"
export default {
  async email(message, env, ctx) {
    await run({ message });
  }
}
"#;

#[wasm_bindgen]
pub fn compile(input: &str) -> Result<String, String> {
    let out = sieve2workers::compile_sieve_to_js(input)
        .map_err(|err| format!("failed to compile: {err}"))?;

    let out = out + "\n" + WORKER_TEMPLATE;
    Ok(out)
}
