use clap::{Subcommand, Parser};
use anyhow::Result;
use std::convert::TryInto;
use twine_core::resolver::Resolver;

#[derive(Debug, Parser)]
pub struct Command {
  #[arg(short, long)]
  pub resolver: Option<String>,
}

impl Command {
  // list strands from resolver
  pub fn run(&self, config: &crate::config::Config) -> Result<()> {
    let resolver = match &self.resolver {
      Some(resolver) => config.resolvers.get(resolver).ok_or(anyhow::anyhow!("Resolver not found"))?,
      None => config.resolvers.get_default().ok_or(anyhow::anyhow!("No default resolver set. Please specify a resolver with -r"))?,
    };
    let resolver = resolver.as_resolver()?;
    let strands = resolver.strands()?;
    for strand in strands {
      println!("{}", strand);
    }
    Ok(())
  }
}
