use anyhow::Result;
use clap::{Parser, Subcommand};
mod check;
mod create;
mod init;
mod keygen;
mod list;
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
  /// List strands
  Ls(list::ListCommand),
  /// Manage sync strands
  Sync(sync::SyncCommand),
  /// Create a strand
  Create(create::CreateCommand),
  /// Generate a keypair
  Keygen(keygen::KeygenCommand),
  /// Initialize a new configuration and store
  Init(init::InitCommand),
  /// Check strand connectivity
  Check(check::CheckCommand),
}

impl Cli {
  pub async fn run(&self, ctx: crate::Context) -> Result<()> {
    match &self.subcommand {
      SubCommands::Ls(ls) => ls.run(ctx).await,
      SubCommands::Sync(sync) => sync.run(ctx).await,
      SubCommands::Create(create) => create.run(ctx).await,
      SubCommands::Keygen(keygen) => keygen.run(ctx).await,
      SubCommands::Init(init) => init.run(ctx).await,
      SubCommands::Check(check) => check.run(ctx).await,
    }
  }
}
