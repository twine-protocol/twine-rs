use std::str::FromStr;
use clap::Parser;
use anyhow::Result;
use twine_core::Cid;

#[derive(Debug, Parser)]
pub struct SyncCommand {
  pub strand: Option<String>,
}

impl SyncCommand {
  pub async fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    match self.strand {
      Some(ref strand) => {
        config.sync_strands.push(Cid::from_str(strand)?);
        log::info!("Synchronizing changes from strand: {}", strand);
        config.save()?;
      },
      None => {
        println!("Syncing {} strands", config.sync_strands.len());
        for strand in &config.sync_strands {
          println!("{}", strand);
        }
      }
    }
    Ok(())
  }
}



#[derive(Debug, Parser)]
pub struct UnSyncCommand {
  pub strand: String,
}

impl UnSyncCommand {
  pub async fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    let cid = Cid::from_str(&self.strand)?;
    config.sync_strands.retain(|s| s != &cid);
    log::info!("No longer synchronizing strand: {}", self.strand);
    config.save()?;
    Ok(())
  }
}
