use crate::{prompt::prompt_for_directory, stores::StoreUri};
use anyhow::Result;
use clap::Parser;
use inquire::Select;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Parser)]
pub struct InitCommand {}

impl InitCommand {
  pub async fn run(&self, ctx: crate::Context) -> Result<()> {
    // if we don't have a config, create one.
    let mut cfg = match ctx.cfg {
      Some(cfg) => cfg,
      None => {
        log::info!("No config found, creating a new one");
        crate::config::Config::load_or_create_local()?
      }
    };

    if cfg.store.is_some() {
      log::info!("Store already configured, skipping init");
      return Ok(());
    }

    enum StoreType {
      PickleDb,
      Sled,
      Car,
    }

    let store_types: HashMap<&str, StoreType> = HashMap::from_iter(vec![
      ("PickleDb", StoreType::PickleDb),
      ("Sled", StoreType::Sled),
      ("Car", StoreType::Car),
    ]);

    let store_type =
      Select::new("Select store type", store_types.keys().cloned().collect()).prompt()?;

    let store_path = prompt_for_directory("Path to store data:", "./")?;

    let store_cfg = match store_types.get(&store_type).unwrap() {
      StoreType::Car => StoreUri {
        scheme: "car".to_string(),
        path: PathBuf::from(store_path)
          .join("store.car")
          .to_string_lossy()
          .to_string(),
      },
      StoreType::Sled => StoreUri {
        scheme: "sled".to_string(),
        path: PathBuf::from(store_path)
          .join("store.sled")
          .to_string_lossy()
          .to_string(),
      },
      StoreType::PickleDb => StoreUri {
        scheme: "pickledb".to_string(),
        path: PathBuf::from(store_path)
          .join("store.pickle")
          .to_string_lossy()
          .to_string(),
      },
    };

    cfg.store = Some(store_cfg.into());
    let _store = cfg.get_store()?.unwrap();

    cfg.save()?;

    Ok(())
  }
}
