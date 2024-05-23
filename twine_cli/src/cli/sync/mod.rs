use std::{hash::RandomState, str::FromStr};
use clap::Parser;
use anyhow::Result;
use clap_stdin::MaybeStdin;
use twine_core::Cid;

#[derive(Debug, Parser)]
pub struct SyncCommand {
  pub strands: Option<Vec<MaybeStdin<String>>>,
}

impl SyncCommand {
  pub async fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
    match self.strands {
      Some(ref strands) => {
        let set = strands.iter()
          .flat_map(|s| s.split_whitespace())
          .map(|s| Cid::from_str(&s).map_err(|e| anyhow::anyhow!(e)))
          .collect::<Result<std::collections::HashSet<_, RandomState>>>()?;
        config.sync_strands = &config.sync_strands | &set;
        log::info!("Now synchronizing changes from strands: {}", set.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "));
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
  pub async fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
    let cid = Cid::from_str(&self.strand)?;
    config.sync_strands.retain(|s| s != &cid);
    log::info!("No longer synchronizing strand: {}", self.strand);
    config.save()?;
    Ok(())
  }
}
