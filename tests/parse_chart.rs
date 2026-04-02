use std::io::Write;
use std::process::Command;

use aski_rs::codegen::CodegenConfig;
use aski_rs::compiler::compile_directory;

#[test]
fn parse_chart_with_aski_parser() {
    let config = CodegenConfig { rkyv: false };
    let generated = compile_directory(
        &[
            "aski/token.aski",
            "aski/tokens.aski",
            "aski/parser.aski",
            "aski/main.aski",
        ],
        &config,
    )
    .expect("failed to compile aski-cc");

    // Remove generated main — we provide our own
    let generated = generated.replace("fn main() {", "fn _gen_main() {");

    // No FFI stubs — everything is pure aski
    assert!(!generated.contains("todo!()"), "should have no FFI stubs:\n{generated}");

    // Lex chart.aski
    let chart_source = std::fs::read_to_string("../astro-aski/aski/chart.aski")
        .expect("failed to read chart.aski");
    let spanned = aski_rs::lexer::lex(&chart_source).expect("lex failed");

    let mut token_lits = Vec::new();
    for st in &spanned {
        use aski_rs::lexer::Token as RT;
        let lit = match &st.token {
            RT::LParen => "Token::LParen",
            RT::RParen => "Token::RParen",
            RT::LBracket => "Token::LBracket",
            RT::RBracket => "Token::RBracket",
            RT::LBrace => "Token::LBrace",
            RT::RBrace => "Token::RBrace",
            RT::CompositionOpen => "Token::CompositionOpen",
            RT::CompositionClose => "Token::CompositionClose",
            RT::Dot => "Token::Dot",
            RT::At => "Token::At",
            RT::Colon => "Token::Colon",
            RT::Tilde => "Token::Tilde",
            RT::Bang => "Token::Bang",
            RT::Caret => "Token::Caret",
            RT::Underscore => "Token::Underscore",
            RT::Newline => "Token::Newline",
            RT::Pipe => "Token::Pipe",
            RT::PascalIdent(s) => {
                token_lits.push(format!("Token::PascalIdent(\"{s}\".to_string())"));
                continue;
            }
            RT::CamelIdent(s) => {
                token_lits.push(format!("Token::CamelIdent(\"{s}\".to_string())"));
                continue;
            }
            _ => "Token::Newline",
        };
        token_lits.push(lit.to_string());
    }

    let test_code = format!(r#"
{generated}

fn main() {{
    // Mutually-recursive aski methods need larger stack in debug mode
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(_real_main)
        .unwrap()
        .join()
        .unwrap();
}}

fn _real_main() {{
    let tokens = vec![
        {}
    ];
    println!("Lexed {{}} tokens from chart.aski", tokens.len());

    let t = Tokens {{ stream: tokens, pos: 0 }};

    // Skip module header: ( ... )
    let t = t.skip_newlines();
    let t = if t.peek_is_l_paren() {{
        let mut t2 = t.advance();
        while !t2.at_end() && !t2.peek_is_r_paren() {{
            t2 = t2.advance();
        }}
        if !t2.at_end() {{ t2.advance() }} else {{ t2 }}
    }} else {{ t }};

    // Parse all items using the aski parser
    let count = t.parse_all_items(&0);
    println!("Parsed {{}} items from chart.aski", count);
}}
"#, token_lits.join(",\n        "));

    let dir = std::env::temp_dir();
    let rs_path = dir.join("aski_cc_chart_test.rs");
    let bin_path = dir.join("aski_cc_chart_test_bin");
    {
        let mut f = std::fs::File::create(&rs_path).expect("create");
        f.write_all(test_code.as_bytes()).expect("write");
    }

    let output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("rustc");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "rustc failed:\n{stderr}");

    let run = Command::new(&bin_path).output().expect("run");
    let stdout = String::from_utf8_lossy(&run.stdout);
    eprintln!("Output:\n{stdout}");

    // chart.aski has 13 items: 7 domains + 3 structs + 3 impl blocks
    assert!(
        stdout.contains("Parsed 13 items"),
        "should parse all 13 items from chart.aski:\n{stdout}"
    );

    let _ = std::fs::remove_file(&rs_path);
    let _ = std::fs::remove_file(&bin_path);
}
