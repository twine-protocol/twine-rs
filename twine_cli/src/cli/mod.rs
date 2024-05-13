use anyhow::Result;
use clap::{Subcommand, Parser};
mod resolver;
mod list;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
  #[command(subcommand)]
  pub subcommand: SubCommands,
  #[arg(short, long, action = clap::ArgAction::Count, global = true)]
  pub verbose: u8,
  #[arg(short, long, global = true)]
  pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
  // twine resolver add URI --name NAME
  Resolver(resolver::Command),
  // twine ls --resolver URI_OR_NAME
  Ls(list::Command),
}

impl Cli {
  pub fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    match &self.subcommand {
      SubCommands::Resolver(resolver) => {
        resolver.run(config)
      },
      SubCommands::Ls(ls) => {
        ls.run(config)
      },
    }
  }
}
