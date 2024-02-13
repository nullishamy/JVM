extern crate proc_macro;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::token::Comma;
use syn::parse_macro_input;
use syn::{Ident, LitStr, Result};

struct JavaInput {
    name: Ident,
    _sep: Comma,
    code: LitStr,
}

impl Parse for JavaInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(JavaInput {
            name: input.parse()?,
            _sep: input.parse()?,
            code: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn java(input: TokenStream) -> TokenStream {
    let tmp_dir: PathBuf = env::var("CARGO_TARGET_TMPDIR")
        .expect("the tmp dir to be set")
        .into();

    let input = parse_macro_input!(input as JavaInput);
    let path = tmp_dir.join(input.name.to_string());

    let class_path = tmp_dir.join(format!("{}.java", input.name));
    let mut class_file = File::create(class_path).expect("file creation to succeed");
    class_file
        .write_all(input.code.value().as_bytes())
        .expect("file to write");

    // https://stackoverflow.com/questions/2441760/differences-between-classpath-and-sourcepath-options-of-javac
    // using the classpath so that javac will not attempt to recompile our utils (which are pre-compiled by a utility and placed into)
    // tmp_dir/kate so that our tests can reference them
    let compilation = Command::new("javac")
        .args(["-cp", &tmp_dir.display().to_string()])
        .arg(tmp_dir.join(format!("{}.java", path.display())))
        .args(["--add-exports", "java.base/jdk.internal.ref=ALL-UNNAMED"])
        .args(["--add-exports", "java.base/jdk.internal.misc=ALL-UNNAMED"])
        .arg("-d")
        .arg(&tmp_dir)
        .output()
        .expect("compilation to succeed");

    if !compilation.status.success() {
        let (stdout, stderr) = (
            String::from_utf8(compilation.stdout).expect("stdout to be valid utf8"),
            String::from_utf8(compilation.stderr).expect("stdout to be valid utf8"),
        );

        panic!("compilation failed.\n{}\n{}", stdout, stderr);
    }

    let classfile_path = tmp_dir.join(format!("{}.class", input.name));
    let bytes = fs::read(classfile_path).expect("classfile to be read");

    let val = bytes.iter().map(|b| format!("{}_u8", b.to_string())).collect::<Vec<_>>();

    let array_lit = format!("(&[{}], \"{}\".to_string())", val.join(", "), input.name);
    array_lit.parse().unwrap()
}
