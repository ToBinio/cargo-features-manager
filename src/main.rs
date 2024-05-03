#![warn(clippy::unwrap_used)]

use std::io;
use std::process::exit;

use clap::{arg, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use color_eyre::Result;
use console::Term;

use crate::document::Document;
use crate::prune::prune;
use crate::rendering::display::Display;

mod dependencies;
mod document;

mod prune;
mod rendering;

mod parsing;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Features(FeaturesArgs),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct FeaturesArgs {
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,

    #[arg(long, short)]
    dependency: Option<String>,

    #[command(subcommand)]
    sub: Option<FeaturesSubCommands>,
}

#[derive(Subcommand)]
enum FeaturesSubCommands {
    Prune {
        #[arg(long, short)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let CargoCli::Features(args) = CargoCli::parse();

    if let Some(generator) = args.generator {
        let cmd = &mut FeaturesArgs::command();
        eprintln!("Generating completion file for {generator:?}...");
        generate(
            generator,
            cmd,
            cmd.get_name().to_string(),
            &mut io::stdout(),
        );
        return Ok(());
    }

    run(args)
}

fn run(args: FeaturesArgs) -> Result<()> {
    let document = Document::new()?;

    if let Some(sub) = args.sub {
        match sub {
            FeaturesSubCommands::Prune { dry_run } => {
                prune(document, dry_run)?;
            }
        }
    } else {
        let mut display = Display::new(document)?;

        if let Some(name) = args.dependency {
            display.set_selected_dep(name)?
        }

        let _ = ctrlc::set_handler(|| {
            let term = Term::stdout();
            term.show_cursor().expect("could not enable cursor");

            exit(0);
        });

        display.start()?;
    }

    Ok(())
}
