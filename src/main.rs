#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::nursery,
    clippy::suspicious,
    clippy::style
)]
#![allow(
    clippy::semicolon_inside_block,
    clippy::just_underscores_and_digits
)]

mod compiler;
use compiler::frontend::*;
mod args;
use args::{Args, Parser};
mod panic;
use std::process::exit;

fn main() {
    panic::init();

    let args = Args::parse();

    #[cfg(debug_assertions)]
    if let Some(msg) = args.panics {
        std::panic::panic_any(format!("{msg}"));
    }

    let filename = &args.input;

    let src = std::fs::read_to_string(filename).unwrap();

    macro_rules! unwrap_or_report {
        ($i: expr) => {{
            let i = $i;
            let (msg, err) = report(i.1, filename, &src, &args);
            print!("{}", msg);
            if err {
                exit(1);
            }

            i.0
        }};
    }

    let mut l = Token::lexer(&src);
    let mut buf = unwrap_or_report!(to_atoken_buf(&mut l));
    let mut pp = unwrap_or_report!(preprocess(&mut buf));

    let ast = unwrap_or_report!(parse(&mut pp, &src));
    println!("{ast:#?}");

    let (nast, ncnt) = unwrap_or_report!(numerate(ast));
    println!("{nast:#?}");

    let types = unwrap_or_report!(typecheck(&nast, ncnt));
    println!("{types:#?}");

    // compiler::backend::llvm::emit_code();
}
