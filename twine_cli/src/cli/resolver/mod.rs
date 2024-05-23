use clap::{Subcommand, Parser};
use anyhow::Result;

fn print_resolvers(resolvers: &crate::config::Resolvers) {
  if resolvers.len() == 0 {
    println!("No resolvers configured");
    return;
  }
  for (index, resolver) in resolvers.iter().enumerate() {
    let default = if resolver.default {
      " (default)"
    } else {
      ""
    };
    println!("[ {} ] (p{}) {}{}{}", index, resolver.priority.unwrap_or(0), resolver.uri, resolver.name.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default(), default);
  }
}

#[derive(Debug, Parser)]
pub struct ResolverCommand {
  #[command(subcommand)]
  pub subcommand: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  Add(add::AddCommand),
  #[clap(alias = "rm")]
  Remove(remove::RemoveCommand),
  #[clap(alias = "ls")]
  List(list::ListCommand),
}

impl ResolverCommand {
  pub fn run(&self, config: &mut crate::config::Config, ctx: crate::Context) -> Result<()> {
    match &self.subcommand {
      Commands::Add(add) => {
        add.run(config, ctx)
      },
      Commands::Remove(remove) => {
        remove.run(config, ctx)
      },
      Commands::List(list) => {
        list.run(config, ctx)
      },
    }
  }
}

mod add {
  use super::*;

  #[derive(Debug, Parser)]
  pub struct AddCommand {
    /// URI of the resolver (e.g. "http://localhost:8080/api/v0")
    pub uri: String,
    /// Optional name for the resolver
    #[arg(short, long)]
    pub name: Option<String>,
    /// Set this resolver as the default
    #[arg(short, long)]
    pub default: bool,
    /// Priority of the resolver (higher priority resolves earlier)
    #[arg(short, long)]
    pub priority: Option<u8>,
  }

  impl AddCommand {
    pub fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
      config.resolvers.add_resolver(self.uri.clone(), self.name.clone(), self.priority, self.default)?;
      Ok(())
    }
  }
}

mod remove {
  use super::*;

  #[derive(Debug, Parser)]
  pub struct RemoveCommand {
    pub uri: String,
  }

  impl RemoveCommand {
    pub fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
      config.resolvers.remove_resolver(&self.uri)?;
      log::info!("Removed resolver {}", self.uri);
      Ok(())
    }
  }
}

mod list {
  use super::*;

  #[derive(Debug, Parser)]
  pub struct ListCommand;

  impl ListCommand {
    pub fn run(&self, config: &crate::config::Config, _ctx: crate::Context) -> Result<()> {
      print_resolvers(&config.resolvers);
      Ok(())
    }
  }
}
