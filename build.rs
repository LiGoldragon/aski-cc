use std::io::Write;

use aski_rs::codegen::CodegenConfig;
use aski_rs::compiler::compile_directory;

fn main() {
    // Rerun if any aski source changes
    println!("cargo:rerun-if-changed=aski/");

    let config = CodegenConfig { rkyv: false };
    let rust_code = compile_directory(
        &[
            "aski/token.aski",
            "aski/tokens.aski",
            "aski/parser.aski",
            "aski/main.aski",
        ],
        &config,
    )
    .expect("failed to compile aski-cc from aski source");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = format!("{out_dir}/aski_cc_generated.rs");
    let mut f = std::fs::File::create(&out_path).expect("failed to create output");
    f.write_all(rust_code.as_bytes()).expect("failed to write");
}
