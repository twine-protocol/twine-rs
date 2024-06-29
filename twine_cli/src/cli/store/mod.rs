use clap::{Subcommand, Parser};
use anyhow::Result;

fn print_stores(stores: &crate::config::Stores) {
  if stores.len() == 0 {
    println!("No stores configured");
    return;
  }
  for (index, store) in stores.iter().enumerate() {
    let default = if store.default {
      " (default)"
    } else {
      ""
    };
    println!("[ {} ] {}{}{}", index, store.uri, store.name.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default(), default);
  }
}

#[derive(Debug, Parser)]
pub struct StoreCommand {
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

impl StoreCommand {
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
    /// URI of the store (e.g. "http://localhost:8080/api/v0")
    pub uri: String,
    /// Optional name for the store
    #[arg(short, long)]
    pub name: Option<String>,
    /// Set this store as the default remote store for push
    #[arg(short, long)]
    pub default: bool,
  }

  impl AddCommand {
    pub fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
      config.stores.add_store(self.uri.clone(), self.name.clone(), self.default)?;
      Ok(())
    }
  }
}

mod remove {
  use super::*;

  #[derive(Debug, Parser)]
  pub struct RemoveCommand {
    pub uri_or_name: String,
  }

  impl RemoveCommand {
    pub fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
      config.stores.remove_store(&self.uri_or_name)?;
      log::info!("Removed store {}", self.uri_or_name);
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
      print_stores(&config.stores);
      Ok(())
    }
  }
}
