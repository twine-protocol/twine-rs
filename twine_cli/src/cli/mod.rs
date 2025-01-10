use anyhow::Result;
use clap::{Subcommand, Parser};
mod resolver;
mod store;
mod list;
mod pull;
mod sync;
mod create;
mod strand;
mod keygen;

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
  #[clap(alias = "resolvers")]
  Resolver(resolver::ResolverCommand),
  /// Manage stores
  #[clap(alias = "stores")]
  Store(store::StoreCommand),
  /// List strands
  Ls(list::ListCommand),
  /// Retrieve and store twines locally
  Pull(pull::PullCommand),
  /// Manage sync strands
  Sync(sync::SyncCommand),
  /// Unsync a strand
  Unsync(sync::UnSyncCommand),
  /// Create a strand
  Create(create::CreateCommand),
  /// Show local strands
  #[clap(alias = "strands")]
  Strand(strand::StrandCommand),
  /// Generate a keypair
  Keygen(keygen::KeygenCommand),
}

impl Cli {
  pub async fn run(&self, config: &mut crate::config::Config, ctx: crate::Context) -> Result<()> {
    match &self.subcommand {
      SubCommands::Resolver(resolver) => {
        resolver.run(config, ctx)
      },
      SubCommands::Store(store) => {
        store.run(config, ctx)
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
      SubCommands::Unsync(unsync) => {
        unsync.run(config, ctx).await
      },
      SubCommands::Create(create) => {
        create.run(config, ctx).await
      },
      SubCommands::Strand(strand) => {
        strand.run(config, ctx).await
      },
      SubCommands::Keygen(keygen) => {
        keygen.run(config, ctx).await
      },
    }
  }
}
