use std::path::PathBuf;

use clap::Parser;

/// Generator for a specific kind of macro im using that counts the amount
/// of arguments and dispatches the apropriate x-macro with said arguments.
#[derive(Parser, Debug)]
pub struct Arguments {

    #[arg(short, long)]
    pub input:  PathBuf,

    #[arg(short, long)]
    pub output: Option<PathBuf>,

    #[arg(short, long)]
    pub logging: bool

}