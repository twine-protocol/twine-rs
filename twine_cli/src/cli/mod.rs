use anyhow::Result;
use clap::{Subcommand, Parser};
mod resolver;
mod list;
mod pull;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
  #[command(subcommand)]
  pub subcommand: SubCommands,
  /// Increase verbosity
  #[arg(short, long, action = clap::ArgAction::Count, global = true)]
  pub verbose: u8,
  /// Suppress all output
  #[arg(short, long, global = true)]
  pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
  /// Manage resolvers
  Resolver(resolver::ResolverCommand),
  /// List strands
  Ls(list::ListCommand),
  /// Retrieve and store twines locally
  Pull(pull::PullCommand),
}

impl Cli {
  pub async fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    match &self.subcommand {
      SubCommands::Resolver(resolver) => {
        resolver.run(config)
      },
      SubCommands::Ls(ls) => {
        ls.run(config).await
      },
      SubCommands::Pull(pull) => {
        pull.run(config).await
      },
    }
  }
}
