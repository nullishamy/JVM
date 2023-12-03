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
    pub const TEST_INIT: &str = "test.init";
    pub const TEST_BOOT: &str = "test.boot";
    pub const TEST_THROW_INTERNAL: &str = "test.throwinternal";
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

    // TODO: Make this a short("cp") once we can: https://github.com/clap-rs/clap/issues/2468
    #[arg(long("cp"))]
    /// A list of paths to add to the classpath
    pub classpath: Vec<String>,
}

impl Cli {
    pub fn has_option(&self, key: &str) -> bool {
        self.options.iter().any(|o| o.key == key)
    }

    pub fn has_option_value(&self, key: &str, value: &str) -> bool {
        self.options.iter().any(|o| o.key == key && o.value == value)
    }
}

#[derive(Subcommand)]
pub enum Command {}
