use assert_cmd::{assert::Assert, Command};

use super::CompiledClass;

const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const SOURCE_DIR: &str = env!("CARGO_MANIFEST_DIR");

pub fn input() -> IntegrationInput {
    IntegrationInput {
        init_std: false,
        opts: vec![],
        stdin: vec![],
        args: vec![],
    }
}

#[derive(Debug)]
pub struct IntegrationInput {
    init_std: bool,
    opts: Vec<(&'static str, &'static str)>,
    args: Vec<String>,
    stdin: Vec<String>,
}

impl IntegrationInput {
    pub fn with_std(mut self) -> Self {
        self.init_std = true;
        self
    }

    pub fn stdin(mut self, line: impl Into<String>) -> Self {
        self.stdin.push(line.into());
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn opt(mut self, key: &'static str, value: &'static str) -> Self {
        self.opts.push((key, value));
        self
    }
}

pub fn exec_integration<const N: usize>(state: IntegrationInput, class: impl Into<CompiledClass<N>>) -> Assert {
    let class = class.into();
    let mut cargo_cmd = Command::cargo_bin("cli").expect("cargo to locate cli");

    let cmd = cargo_cmd
        .arg("--cp")
        .arg(TMP_DIR)
        .arg("--std")
        .arg(format!("{SOURCE_DIR}/../std.jar"))
        .arg("-Xtest.init=true")
        .arg(class.name());

    if state.init_std {
        cmd.arg("-Xtest.boot=true");
    }

    for (key, value) in state.opts {
        cmd.arg(format!("-X{}={}", key, value));
    }

    cmd.arg("--").args(state.args);

    for line in state.stdin {
        cmd.write_stdin(line);
    }

    cmd.assert()
}
