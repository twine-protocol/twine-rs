use clap::{Subcommand, Parser};

#[derive(Debug, Parser)]
pub struct Command {
  #[command(subcommand)]
  pub subcommand: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  Add(add::Command),
}

impl Command {
  pub fn run(&self, config: &mut crate::config::Config) {
    match &self.subcommand {
      Commands::Add(add) => {
        add.run(config);
      }
    }
  }
}

mod add {
  use super::*;


  #[derive(Debug, Parser)]
  pub struct Command {
    pub uri: String,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long)]
    pub default: bool,
  }

  impl Command {
    pub fn run(&self, config: &mut crate::config::Config) {
      config.add_resolver(self.uri.clone(), self.name.clone(), self.default);
      log::info!("Added {} resolver with name {}", self.uri, self.name.as_deref().unwrap_or(""));
    }
  }
}
