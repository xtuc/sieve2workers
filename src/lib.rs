mod codegen;

pub(crate) type BoxError = Box<dyn std::error::Error>;

pub fn compile_sieve_to_js(contents: &str) -> Result<String, BoxError> {
    let compiler = sieve::Compiler::new();
    let script = compiler.compile(contents.as_bytes()).unwrap();

    let js = {
        let opts = codegen::GenerateOpts {
            debug: false,
            vacation_from_address: None,
        };
        let mut code_gen = codegen::CodeGen::new(opts, &script.instructions);
        code_gen.generate_js().unwrap()
    };

    Ok(js)
}
