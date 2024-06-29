use std::{collections::HashSet, hash::RandomState, str::FromStr};
use clap::Parser;
use anyhow::Result;
use clap_stdin::MaybeStdin;
use twine_core::Cid;
use crate::{cid_str::CidStr, config::StrandRecord};

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
          .map(|s| CidStr::from_str(&s).map_err(|e| anyhow::anyhow!(e)))
          .map(|cid| Ok(StrandRecord { cid: cid?, sync: true, key: None, name: None }))
          .collect::<Result<HashSet<_, RandomState>>>()?;

        let (old, to_update): (HashSet<_>, HashSet<_>) = config.strands.iter()
          .cloned()
          .partition(|s| set.contains(&s));
        let to_update = to_update.iter()
          .cloned()
          .map(|s| StrandRecord { sync: true, ..s })
          .collect();
        let new = set.difference(&to_update).cloned().collect::<Vec<_>>();

        config.strands = old.union(&to_update).cloned().chain(new).collect();
        config.save()?;
        log::info!("Now synchronizing changes from strands: {}", set.iter().map(|s| s.cid.to_string()).collect::<Vec<_>>().join(", "));
      },
      None => {
        let synced = config.strands.iter()
          .filter(|s| s.sync);

        println!("Syncing {} strands", synced.clone().count());
        for strand in synced {
          println!("{}", strand.cid);
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
    let record = config.strands.iter()
      .find(|s| s.cid == cid)
      .ok_or_else(|| anyhow::anyhow!("No strand found with CID: {}", self.strand))?;
    let record = StrandRecord { sync: false, ..record.clone() };
    config.strands.replace(record);
    config.save()?;
    log::info!("No longer synchronizing strand: {}", self.strand);
    Ok(())
  }
}
