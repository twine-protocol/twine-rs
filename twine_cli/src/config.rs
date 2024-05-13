use std::collections::HashMap;
use serde::{de, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Resolver {
  pub uri: String,
  pub name: Option<String>,
  pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Resolvers(Vec<Resolver>);

impl Default for Resolvers {
  fn default() -> Self {
    Self(Vec::new())
  }
}

impl Resolvers {
  pub(crate) fn add_resolver(&mut self, uri: String, name: Option<String>, default: bool) {
    self.0.iter_mut().for_each(|r| r.default = false);
    self.0.push(Resolver { uri, name, default });
  }

  pub(crate) fn get_default(&self) -> Option<&Resolver> {
    self.0.iter().find(|r| r.default)
  }

  pub(crate) fn get(&self, name: &str) -> Option<&Resolver> {
    self.0.iter().find(|r| r.name.as_deref() == Some(name))
  }

  pub(crate) fn get_by_uri(&self, uri: &str) -> Option<&Resolver> {
    self.0.iter().find(|r| r.uri == uri)
  }

  pub(crate) fn set_default(&mut self, name: &str) {
    self.0.iter_mut().for_each(|r| r.default = r.name.as_deref() == Some(name));
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
  resolvers: Resolvers,
  default_resolver: Option<String>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      resolvers: Resolvers::default(),
      default_resolver: None,
    }
  }
}

impl Config {
  pub(crate) fn add_resolver(&mut self, uri: String, name: Option<String>, default: bool) {
    self.resolvers.add_resolver(uri, name, default);
  }

  pub(crate) fn save(&self) -> Result<(), anyhow::Error> {
    confy::store("twine_cli", Some("config"), self)?;
    Ok(())
  }
}

pub(crate) fn load_config() -> Result<Config, anyhow::Error> {
  Ok(confy::load("twine_cli", Some("config"))?)
}
