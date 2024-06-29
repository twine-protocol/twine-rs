use pkcs8::{LineEnding, SecretDocument};
use anyhow::Result;

#[derive(Debug)]
pub struct KeyStore {
  dir: std::path::PathBuf,
}

impl KeyStore {
  pub fn new<T: Into<std::path::PathBuf>>(dir: T) -> Self {
    Self { dir: dir.into() }
  }

  pub fn load_keypair<S: AsRef<str>>(&self, name: S) -> Result<SecretDocument> {
    let filename = format!("{}.pem", name.as_ref());
    let path = self.dir.join(filename);
    Ok(SecretDocument::read_pem_file(path).map(|(_, d)| d)?)
  }

  pub fn save_keypair<S: AsRef<str>>(&self, name: S, keypair: &SecretDocument) -> Result<()> {
    let filename = format!("{}.pem", name.as_ref());
    let path = self.dir.join(filename);
    keypair.write_pem_file(
      path,
      "PRIVATE KEY",
      LineEnding::default()
    )?;
    Ok(())
  }

  pub fn keynames(&self) -> Result<Vec<String>> {
    let mut keys = vec![];
    for entry in std::fs::read_dir(&self.dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.is_file() {
        if let Some(ext) = path.extension() {
          if ext == "pem" {
            if let Some(name) = path.file_stem() {
              keys.push(name.to_string_lossy().to_string());
            }
          }
        }
      }
    }
    Ok(keys)
  }
}
