use clap::Parser;
use std::fs;
use std::io::prelude::*;

mod codegen;

pub(crate) type BoxError = Box<dyn std::error::Error>;

/// Sieve for Cloudflare Email Routing
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Sieve file to convert
    input: String,

    /// JS output
    /// By default, writes the output to input + .js
    output: Option<String>,

    #[arg(long, default_value_t = false)]
    /// Generate debug code in the Cloudflare Worker
    debug: bool,
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let contents = fs::read(&args.input)?;

    let compiler = sieve::Compiler::new();
    let script = compiler.compile(&contents).unwrap();

    let js = {
        let opts = codegen::GenerateOpts { debug: args.debug };
        let mut code_gen = codegen::CodeGen::new(opts, &script.instructions);
        code_gen.generate_js().unwrap()
    };

    let out_file = if let Some(output) = &args.output {
        output.to_owned()
    } else {
        args.input + ".js"
    };
    let mut file = fs::File::create(out_file)?;
    file.write_all(js.as_bytes())?;

    Ok(())
}
