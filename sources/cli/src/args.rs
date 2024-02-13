use std::{error, str::FromStr};

use clap::{Parser, Subcommand};

#[derive(Clone)]
pub struct VmOption {
    pub key: String,
    pub value: String,
}

impl FromStr for VmOption {
    type Err = Box<dyn error::Error + Send + Sync + 'static>;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let separator_pos = input
            .find('=')
            .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{input}`"))?;

        Ok(VmOption {
            key: input[..separator_pos].parse()?,
            value: input[separator_pos + 1..].parse()?,
        })
    }
}

// Known vm option keys go here. Allows easy renaming & stops typos.
pub mod opts {
    pub mod test {
        pub const INIT: &str = "test.init";
        pub const BOOT: &str = "test.boot";
        pub const THROW_INTERNAL: &str = "test.throwinternal";
        pub const PANIC_INTERNAL: &str = "test.panicinternal";
    }

    pub mod vm {
        pub const MAX_STACK: &str = "vm.maxstack";
    }

}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// The classes to execute
    pub classes: Vec<String>,

    #[arg(short('X'))]
    pub options: Vec<VmOption>,

    /// The std root to use
    #[arg(long("std"))]
    pub std: Option<String>,

    // TODO: Make this a short("cp") once we can: https://github.com/clap-rs/clap/issues/2468
    #[arg(long("cp"))]
    /// A list of paths to add to the classpath
    pub classpath: Vec<String>,

    #[arg(last = true)]
    /// The arguments to pass to the main function, passed as `cli [opts] -- [extras]`
    pub extras: Vec<String>,
}

impl Cli {
    pub fn has_option(&self, key: &str) -> bool {
        self.get_option(key).is_some()
    }

    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.iter().find(|o| o.key == key).map(|o| &o.value)
    }
}

#[derive(Subcommand)]
pub enum Command {}
