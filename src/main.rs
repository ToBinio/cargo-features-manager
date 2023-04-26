use std::io::stdout;

use clap::{Parser, Subcommand};
use clap::builder::Str;
use crossterm::execute;
use crossterm::style::{Print, Stylize};

use crate::display::Display;

mod crates;
mod display;
mod document;
mod index;

#[derive(Parser)] // requires `derive` feature
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Feature(FeatureArgs),
}

#[derive(clap::Args)]
#[command(author, version, about, long_about = None)]
struct FeatureArgs {
    #[arg(long,short)]
    path: Option<String>,

    #[arg(long,short)]
    dependency: Option<String>,
}

fn main() {
    let args = CargoCli::parse();

    if let Err(err) = Display::run() {
        execute!(
            stdout(),
            Print("error".red().bold()),
            Print(": "),
            Print(err.to_string())
        )
        .unwrap();
    }
}
