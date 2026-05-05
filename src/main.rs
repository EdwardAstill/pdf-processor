mod batch;
mod cli;
mod commands;
mod document;
mod error;
mod figure;
mod formats;
mod hybrid;
mod layout;
mod ocr;
mod pdf;
mod pipeline;
mod processor;
mod render;

use clap::Parser;

use cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    commands::run(cli.into_command()?)
}
