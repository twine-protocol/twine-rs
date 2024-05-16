use serde::{Deserialize, Serialize};
use anyhow::Result;
use twine_core::resolver::BaseResolver;
use twine_http_store::{HttpStore, HttpStoreOptions, reqwest};
use twine_sled_store::{SledStore, SledStoreOptions, sled};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Resolver {
  pub uri: String,
  pub name: Option<String>,
  pub default: bool,
}

impl Resolver {
  pub(crate) fn as_resolver(&self) -> Result<Box<dyn BaseResolver>> {
    match self.uri.split("://").next().unwrap_or_default() {
      "http"|"https" => {
        let cfg = HttpStoreOptions::default()
          .url(&self.uri);
        let r = HttpStore::new(reqwest::Client::new(), cfg);
        Ok(Box::new(r))
      },
      "sled" => {
        let path = self.uri.split_at(5).1;
        let db = sled::Config::new().path(path).open()?;
        let r = SledStore::new(db, SledStoreOptions::default());
        Ok(Box::new(r))
      },
      _ => Err(anyhow::anyhow!("Unknown resolver type: {}", self.uri)),
    }
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

  pub(crate) fn remove_resolver(&mut self, uri_or_name_or_index: &str) -> Result<()> {
    let maybe_index = uri_or_name_or_index.parse::<usize>().ok();
    if let Some(index) = maybe_index {
      if index >= self.0.len() {
        return Err(anyhow::anyhow!("Index out of bounds"));
      }
      self.0.remove(index);
    } else {
      self.0.retain(|r|
        r.uri != uri_or_name_or_index &&
        r.name.as_deref() != Some(uri_or_name_or_index)
      );
    }
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

  pub(crate) fn get_resolver(&self, name_or_uri: &Option<String>) -> Result<Box<dyn BaseResolver>> {
    let r = match name_or_uri {
      Some(resolver) => self.resolvers.get(&resolver).ok_or(anyhow::anyhow!("Resolver not found"))?,
      None => self.resolvers.get_default().ok_or(anyhow::anyhow!("No default resolver set. Please specify a resolver with -r"))?,
    };
    log::trace!("Using resolver: {:?}", r);
    r.as_resolver()
  }

  pub(crate) fn get_local_store(&self) -> Result<SledStore> {
    let proj = directories::ProjectDirs::from("rs", "twine", "twine_cli")
      .ok_or(anyhow::anyhow!("Could not determine local store path"))?;
    let path = proj.data_dir().join("local_store");
    log::trace!("Using local store at: {:?}", path);
    let db = sled::Config::new().path(path).open()?;
    Ok(SledStore::new(db, SledStoreOptions::default()))
  }
}

pub(crate) fn load_config() -> Result<Config> {
  Ok(confy::load("twine_cli", Some("config"))?)
}
