use clap::{Subcommand, Parser};
use anyhow::Result;

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
  pub fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    match &self.subcommand {
      Commands::Add(add) => {
        add.run(config)
      },
      Commands::Remove(remove) => {
        remove.run(config)
      },
      Commands::List(list) => {
        list.run(config)
      },
    }
  }
}

mod add {
  use super::*;

  #[derive(Debug, Parser)]
  pub struct AddCommand {
    pub uri: String,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long)]
    pub default: bool,
  }

  impl AddCommand {
    pub fn run(&self, config: &mut crate::config::Config) -> Result<()> {
      config.resolvers.add_resolver(self.uri.clone(), self.name.clone(), self.default)?;
      match &self.name {
        Some(name) => log::info!("Added resolver {} with name {}", self.uri, name),
        None => log::info!("Added resolver {}", self.uri),
      }
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
    pub fn run(&self, config: &mut crate::config::Config) -> Result<()> {
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
    pub fn run(&self, config: &crate::config::Config) -> Result<()> {
      let default = config.resolvers.get_default().map(|r| r.name.as_deref()).flatten();
      for (index, resolver) in config.resolvers.iter().enumerate() {
        let default = if resolver.name.as_deref() == default {
          " (default)"
        } else {
          ""
        };
        println!("({}) {}{}{}", index, resolver.uri, resolver.name.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default(), default);
      }
      Ok(())
    }
  }
}
