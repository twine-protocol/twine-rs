use crate::prompt::prompt_for_filename;
use anyhow::Result;
use clap::Parser;
use inquire::Select;
use twine_builder::RingSigner;

#[derive(Debug, Parser)]
pub struct KeygenCommand {
  /// Output the private key to a file
  #[arg(short, long)]
  output: Option<String>,
}

impl KeygenCommand {
  pub async fn run(&self, _ctx: crate::Context) -> Result<()> {
    let filename = if self.output.is_none() {
      prompt_for_filename("Filename to save the private key to:", "./key.pem")?
    } else {
      self.output.clone().unwrap()
    };

    let items = vec![
      "Ed25519",
      "EcdsaP256",
      "EcdsaP384",
      "RSA2048 (sha256)",
      "RSA3072 (sha384)",
      "RSA4096 (sha512)",
    ];

    let key_type = Select::new("Select key type", items).prompt()?;

    let signer = match key_type {
      "Ed25519" => RingSigner::generate_ed25519().map_err(|e| anyhow::anyhow!(e))?,
      "EcdsaP256" => RingSigner::generate_p256().map_err(|e| anyhow::anyhow!(e))?,
      "EcdsaP384" => RingSigner::generate_p384().map_err(|e| anyhow::anyhow!(e))?,
      "RSA2048 (sha256)" => RingSigner::generate_rs256(2048)?,
      "RSA3072 (sha384)" => RingSigner::generate_rs384(3072)?,
      "RSA4096 (sha512)" => RingSigner::generate_rs512(4096)?,
      _ => unreachable!(),
    };

    let pem = signer.private_key_pem()?;

    // write the file and set permissions to 600
    tokio::fs::write(&filename, pem).await?;

    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = tokio::fs::metadata(&filename).await?.permissions();
      perms.set_mode(0o600);
      tokio::fs::set_permissions(&filename, perms).await?;
    }

    log::info!("Private key saved to {}", filename);

    Ok(())
  }
}
