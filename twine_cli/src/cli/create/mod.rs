use crate::prompt::not_empty;
use crate::prompt::prompt_for_directory;
use anyhow::Result;
use clap::Parser;
use inquire::{validator::Validation, Select, Text};
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};
use twine_builder::RingSigner;
use twine_core::ipld_core::ipld;
use twine_core::{multihash_codetable::Code, twine::TwineBlock};

#[derive(Debug, Parser)]
pub struct CreateCommand {
  /// Key to sign the strand with
  #[arg(short, long)]
  key: PathBuf,
}

impl CreateCommand {
  pub async fn run(&self, ctx: crate::Context) -> Result<()> {
    // if we don't have a config with a store, we can't save the strand
    if ctx.cfg.is_none() {
      return Err(anyhow::anyhow!(
        "No store configured. Run `twine init` to set a store"
      ));
    }

    let cfg = ctx.cfg.unwrap();

    if cfg.store.is_none() {
      return Err(anyhow::anyhow!(
        "No store configured. Run `twine init` to set a store"
      ));
    }

    let directory = prompt_for_directory("Directory to store strand:", "./my-strand")?;

    // Create the directory if needed
    if !Path::new(&directory).exists() {
      tokio::fs::create_dir_all(&directory).await?;
    }

    let pem = tokio::fs::read_to_string(&self.key).await?;
    let signer =
      RingSigner::from_pem(&pem).map_err(|e| anyhow::anyhow!("Failed to load key. {}", e))?;

    log::info!("Using key with algorithm: {}", signer.alg());

    let builder = twine_builder::TwineBuilder::new(signer);

    // Hash type to use
    let hash_choices: HashMap<&str, Code> = HashMap::from_iter(vec![
      ("Sha3_512", Code::Sha3_512),
      ("Sha3_384", Code::Sha3_384),
      ("Sha3_256", Code::Sha3_256),
      ("Sha3_224", Code::Sha3_224),
      ("Sha2_512", Code::Sha2_512),
      ("Sha2_256", Code::Sha2_256),
      ("Blake3_256", Code::Blake3_256),
    ]);

    let hash_type =
      Select::new("Select hash type", hash_choices.keys().cloned().collect()).prompt()?;

    let description = Text::new("Short public description of the strand:")
      .with_validator(not_empty)
      .with_validator(|text: &str| {
        if text.len() < 120 {
          Ok(Validation::Valid)
        } else {
          Ok(Validation::Invalid(
            "Description must be less than 120 characters".into(),
          ))
        }
      })
      .prompt()?;

    let strand = builder
      .build_strand()
      .hasher(hash_choices.get(&hash_type).unwrap().clone())
      .details(ipld!({
        "description": description,
      }))
      .done()?;

    // write it to a json file in dir
    let strand_file = Path::new(&directory).join(format!("{}.strand.json", strand.cid()));
    let strand_json = strand.tagged_dag_json_pretty();
    tokio::fs::write(&strand_file, strand_json).await?;

    // save the strand to the store
    let store = cfg
      .get_store()?
      .ok_or_else(|| anyhow::anyhow!("No store configured"))?;
    store.save(strand.clone()).await?;

    log::info!("Saved strand {} to {}", strand.cid(), directory);

    Ok(())
  }
}
