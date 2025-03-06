use crate::stores::{AnyStore, StoreUri};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};
use twine_core::resolver::ResolverSetSeries;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoreUriString(#[serde_as(as = "DisplayFromStr")] pub StoreUri);

impl From<StoreUri> for StoreUriString {
  fn from(uri: StoreUri) -> Self {
    Self(uri)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
  #[serde(skip)]
  pub path: Option<PathBuf>,
  pub resolvers: HashMap<String, StoreUriString>,
  pub store: Option<StoreUriString>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      resolvers: HashMap::new(),
      store: None,
      path: None,
    }
  }
}

#[allow(dead_code)]
impl Config {
  pub fn get_named_resolver(&self, name: &str) -> Option<Result<AnyStore>> {
    self
      .resolvers
      .get(name)
      .map(|uri| uri.0.clone())
      .map(|uri| AnyStore::try_from(uri))
  }

  pub fn all_resolvers(&self) -> Result<ResolverSetSeries<AnyStore>> {
    let mut stores = self
      .resolvers
      .values()
      .map(|uri| uri.0.clone())
      .map(|uri| AnyStore::try_from(uri))
      .collect::<Result<Vec<_>>>()?;

    if let Some(store) = &self.store {
      stores.push(AnyStore::try_from(store.0.clone())?);
    }

    Ok(ResolverSetSeries::new(stores))
  }

  pub fn get_store(&self) -> Result<Option<AnyStore>> {
    if let Some(store) = &self.store {
      Ok(Some(AnyStore::try_from(store.0.clone())?))
    } else {
      Ok(None)
    }
  }

  pub fn load_path(path: impl AsRef<Path>) -> Result<Self> {
    let mut config: Self = confy::load_path(path.as_ref())?;
    config.path = Some(path.as_ref().to_path_buf());
    Ok(config)
  }

  pub fn save_path(&self, path: impl AsRef<Path>) -> Result<()> {
    confy::store_path(path, self)?;
    Ok(())
  }

  pub fn load_or_create_local() -> Result<Self> {
    let path = Path::new("./twine.toml");
    Ok(Self::load_path(path)?)
  }

  pub fn load_local() -> Result<Option<Self>> {
    let path = Path::new("./twine.toml");
    if !path.exists() {
      return Ok(None);
    }
    Ok(Some(Self::load_path(path)?))
  }

  pub fn save_local(&self) -> Result<()> {
    let path = Path::new("./twine.toml");
    confy::store_path(path, self)?;
    Ok(())
  }

  pub fn save(&self) -> Result<()> {
    if let Some(path) = &self.path {
      confy::store_path(path, self)?;
    } else {
      unreachable!("Config has no path");
    }
    Ok(())
  }

  pub fn load_global() -> Result<Option<Self>> {
    todo!("load global config")
  }

  pub fn save_global(&self) -> Result<()> {
    todo!("save global config")
  }
}
