use crate::prompt::prompt_for_filename;
use anyhow::Result;
use clap::Parser;
use inquire::Select;
use twine_builder::RustCryptoSigner;
use twine_lib::crypto::SignatureAlgorithm;

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
      "Ed25519" => RustCryptoSigner::generate_ed25519(),
      "EcdsaP256" => RustCryptoSigner::generate_p256(),
      "EcdsaP384" => RustCryptoSigner::generate_p384(),
      "RSA2048 (sha256)" => RustCryptoSigner::generate(SignatureAlgorithm::Sha256Rsa(2048))?,
      "RSA3072 (sha384)" => RustCryptoSigner::generate(SignatureAlgorithm::Sha384Rsa(3072))?,
      "RSA4096 (sha512)" => RustCryptoSigner::generate(SignatureAlgorithm::Sha512Rsa(4096))?,
      _ => unreachable!(),
    };

    let pem = signer.to_pkcs8_pem()?;

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
