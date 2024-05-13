use clap::{Subcommand, Parser};
mod resolver;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
  #[command(subcommand)]
  pub subcommand: SubCommands,
  #[arg(short, long, action = clap::ArgAction::Count)]
  pub verbose: u8,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
  // twine resolver add URI --name NAME
  Resolver(resolver::Command),
}

impl Cli {
  pub fn run(&self, config: &mut crate::config::Config) {
    match &self.subcommand {
      SubCommands::Resolver(resolver) => {
        resolver.run(config);
      }
    }
  }
}
