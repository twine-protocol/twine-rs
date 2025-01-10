use std::os::unix::fs::PermissionsExt;
use clap::Parser;
use anyhow::Result;
use inquire::Select;
use twine_builder::RingSigner;
use crate::prompt::prompt_for_filename;


#[derive(Debug, Parser)]
pub struct KeygenCommand {
  /// Output the private key to a file
  #[arg(short, long)]
  output: Option<String>,
}

impl KeygenCommand {
  pub async fn run(&self, _config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
    let filename = if self.output.is_none() {
      prompt_for_filename("Filename to save the private key to:", "./key.pem")?
    } else {
      self.output.clone().unwrap()
    };

    let items = vec![
      "Ed25519",
      "EcdsaP256",
      "EcdsaP384"
    ];

    let key_type = Select::new("Select key type", items)
      .prompt()?;

    let signer = match key_type {
      "Ed25519" => RingSigner::generate_ed25519(),
      "EcdsaP256" => RingSigner::generate_p256(),
      "EcdsaP384" => RingSigner::generate_p384(),
      _ => unreachable!(),
    };

    let signer = match signer {
      Ok(s) => s,
      Err(e) => return Err(anyhow::anyhow!("Error generating keypair: {}", e)),
    };

    let pem = signer.private_key_pem()?;

    // write the file and set permissions to 600
    tokio::fs::write(&filename, pem).await?;
    let mut perms = tokio::fs::metadata(&filename).await?.permissions();
    perms.set_mode(0o600);
    tokio::fs::set_permissions(&filename, perms).await?;

    log::info!("Private key saved to {}", filename);

    Ok(())
  }

}
