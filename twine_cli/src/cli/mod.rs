use anyhow::Result;
use clap::{Subcommand, Parser};
mod resolver;
mod list;
mod pull;
mod sync;

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
  /// Manage sync strands
  Sync(sync::SyncCommand),
  /// Unsync a strand
  UnSync(sync::UnSyncCommand),
}

impl Cli {
  pub async fn run(&self, config: &mut crate::config::Config, ctx: crate::Context) -> Result<()> {
    match &self.subcommand {
      SubCommands::Resolver(resolver) => {
        resolver.run(config, ctx)
      },
      SubCommands::Ls(ls) => {
        ls.run(config, ctx).await
      },
      SubCommands::Pull(pull) => {
        pull.run(config, ctx).await
      },
      SubCommands::Sync(sync) => {
        sync.run(config, ctx).await
      },
      SubCommands::UnSync(unsync) => {
        unsync.run(config, ctx).await
      },
    }
  }
}
