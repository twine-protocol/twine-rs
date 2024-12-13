use std::{collections::HashSet, hash::Hash, sync::Arc};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use twine_core::{resolver::unchecked_base, Cid};
use twine_sled_store::{SledStore, SledStoreOptions, sled};
use crate::{cid_str::CidStr, PROJECT_DIRS};

mod resolver_config;
pub use resolver_config::*;
mod store_config;
pub use store_config::*;

lazy_static::lazy_static! {
  static ref STORE: Arc<SledStore> = {
    let path = PROJECT_DIRS.data_dir().join("local_store");
    log::trace!("Using local store at: {:?}", path);
    let db = sled::Config::default().path(path).print_profile_on_drop(true).open().expect("Failed to open local store");
    Arc::new(SledStore::new(db, SledStoreOptions::default()))
  };
}


#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub(crate) struct StrandRecord {
  pub name: Option<String>,
  pub cid: CidStr,
  pub key: Option<String>,
  pub sync: bool,
}

impl PartialEq for StrandRecord {
  fn eq(&self, other: &Self) -> bool {
    // if cids are the same
    // OR if they have names that are the same then the record is the same
    self.cid == other.cid ||
      self.name.as_deref().map_or(false, |n| other.name.as_deref() == Some(&n))
  }
}

impl Eq for StrandRecord {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
  pub resolvers: Resolvers,
  pub stores: Stores,
  pub strands: HashSet<StrandRecord>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      resolvers: Resolvers::default(),
      stores: Stores::default(),
      strands: HashSet::new(),
    }
  }
}

impl Config {

  pub(crate) fn get_resolver(&self, name_or_uri: &Option<String>) -> Result<Box<dyn unchecked_base::BaseResolver>> {
    let r = match name_or_uri.as_deref() {
      Some("local") => {
        let store = self.get_local_store()?;
        return Ok(Box::new(store));
      },
      Some(resolver) => {
        match self.resolvers.get(&resolver) {
          Some(r) => r,
          None if resolver.contains("/") => {
            // try to interpret it as a uri
            let r = resolver.parse::<ResolverRecord>()?;
            return r.as_resolver();
          },
          None => {
            return Err(anyhow::anyhow!("Resolver {} not found", resolver));
          }
        }
      },
      None => self.resolvers.get_default().ok_or(anyhow::anyhow!("No default resolver set. Please specify a resolver with -r"))?,
    };
    log::trace!("Using resolver: {:?}", r);
    r.as_resolver()
  }

  pub(crate) fn get_store(&self, name_or_uri: &Option<String>) -> Result<AnyStore> {
    let s = match name_or_uri.as_deref() {
      Some("local") => {
        return Ok(AnyStore::Sled(self.get_local_store()?));
      },
      Some(store) => {
        match self.stores.get(&store) {
          Some(s) => s,
          None if store.contains("/") => {
            // try to interpret it as a uri
            let s = store.parse::<StoreRecord>()?;
            return s.as_store();
          },
          None => {
            return Err(anyhow::anyhow!("Store {} not found", store));
          }
        }
      },
      None => self.stores.get_default().ok_or(anyhow::anyhow!("No default store set. Please specify a store with -s"))?,
    };
    log::trace!("Using store: {:?}", s);
    s.as_store()
  }

  pub(crate) fn get_local_store(&self) -> Result<Arc<SledStore>> {
    Ok(STORE.clone())
  }

  pub(crate) fn sync_strands(&self) -> impl Iterator<Item = Cid> + '_ {
    self.strands.iter()
      .filter(|s| s.sync)
      .map(|s| s.cid.clone().into())
  }

  #[cfg(debug_assertions)]
  pub(crate) fn save(&self) -> Result<()> {
    use std::env::temp_dir;
    let tmpdir = temp_dir();
    confy::store_path(tmpdir.join("config.toml"), self)?;
    Ok(())
  }

  #[cfg(not(debug_assertions))]
  pub(crate) fn save(&self) -> Result<()> {
    confy::store("twine_cli", Some("config"), self)?;
    Ok(())
  }
}

#[cfg(debug_assertions)]
pub(crate) fn load_config() -> Result<Config> {
  use std::env::temp_dir;
  let tmpdir = temp_dir();
  log::debug!("Loading config from: {:?}", tmpdir.join("config.toml"));
  Ok(confy::load_path(tmpdir.join("config.toml"))?)
}

#[cfg(not(debug_assertions))]
pub(crate) fn load_config() -> Result<Config> {
  Ok(confy::load("twine_cli", Some("config"))?)
}
