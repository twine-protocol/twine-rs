use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::multi_resolver::MultiResolver;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Resolver {
  pub uri: String,
  pub name: Option<String>,
  pub default: bool,
}

impl Resolver {
  pub(crate) fn as_resolver(&self) -> Result<MultiResolver> {
    MultiResolver::new_from_string(&self.uri)
  }
}

impl PartialEq for Resolver {
  fn eq(&self, other: &Self) -> bool {
    self.uri == other.uri
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Resolvers(Vec<Resolver>);

impl Default for Resolvers {
  fn default() -> Self {
    Self(Vec::new())
  }
}

impl Resolvers {
  pub(crate) fn add_resolver(&mut self, uri: String, name: Option<String>, default: bool) -> Result<()> {
    self.0.iter_mut().for_each(|r| r.default = false);
    let record = Resolver { uri, name, default };
    // Check if resolver with URI already exists... and update it
    if let Some(r) = self.0.iter_mut().find(|r| **r == record) {
      *r = record;
      return Ok(());
    }
    // Check if resolver with name already exists... and return error. (None doesn't count)
    if let Some(name) = record.name.as_deref() {
      if self.0.iter().any(|r| r.name.as_deref() == Some(name)) {
        return Err(anyhow::anyhow!("Resolver with name {} already exists", name));
      }
    }

    self.0.push(record);
    Ok(())
  }

  pub(crate) fn remove_resolver(&mut self, uri_or_name: &str) -> Result<()> {
    self.0.retain(|r| r.uri != uri_or_name && r.name.as_deref() != Some(uri_or_name));
    Ok(())
  }

  pub(crate) fn get_default(&self) -> Option<&Resolver> {
    self.0.iter().find(|r| r.default)
  }

  pub(crate) fn get(&self, name_or_uri: &str) -> Option<&Resolver> {
    self.0.iter().find(|r| r.name.as_deref() == Some(name_or_uri) || r.uri == name_or_uri)
  }

  pub(crate) fn get_by_uri(&self, uri: &str) -> Option<&Resolver> {
    self.0.iter().find(|r| r.uri == uri)
  }

  pub(crate) fn set_default(&mut self, name: &str) {
    self.0.iter_mut().for_each(|r| r.default = r.name.as_deref() == Some(name));
  }

  pub(crate) fn iter(&self) -> impl Iterator<Item = &Resolver> {
    self.0.iter()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
  pub resolvers: Resolvers,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      resolvers: Resolvers::default(),
    }
  }
}

impl Config {

  pub(crate) fn save(&self) -> Result<()> {
    confy::store("twine_cli", Some("config"), self)?;
    Ok(())
  }
}

pub(crate) fn load_config() -> Result<Config> {
  Ok(confy::load("twine_cli", Some("config"))?)
}
