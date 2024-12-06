use std::collections::HashSet;

use clap::Parser;
use anyhow::Result;
use inquire::{validator::Validation, Confirm, Select, Text};
use twine_builder::RingSigner;
use twine_core::{ipld_core::ipld, resolver::Resolver, store::Store};
use futures::{stream::StreamExt, TryStreamExt};
use crate::config::StrandRecord;
use twine_core::crypto::SignatureAlgorithm;

fn not_empty(text: &str) -> std::result::Result<Validation, Box<dyn std::error::Error + Send + Sync>> {
  if text.is_empty() {
    Ok(Validation::Invalid("This field cannot be empty".into()))
  } else {
    Ok(Validation::Valid)
  }
}

fn only_simple_chars(text: &str) -> std::result::Result<Validation, Box<dyn std::error::Error + Send + Sync>> {
  if text.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
    Ok(Validation::Valid)
  } else {
    Ok(Validation::Invalid("Only alphanumeric characters, dashes, and underscores are allowed".into()))
  }
}

#[derive(Debug, Parser)]
pub struct CreateCommand {
}

impl CreateCommand {
  pub async fn run(&self, config: &mut crate::config::Config, ctx: crate::Context) -> Result<()> {
    let store = config.get_local_store()?;

    let existing_names = config.strands.iter().filter_map(|r| r.name.clone()).collect::<HashSet<_>>();
    let nickname = Text::new("Nickname for the strand:")
      .with_validator(not_empty)
      .with_validator(only_simple_chars)
      .with_validator(move |text: &str| {
        // Check if the nickname is already in use
        if existing_names.contains(&text.to_string()) {
          Ok(Validation::Invalid("You already own a strand with this nickname".into()))
        } else {
          Ok(Validation::Valid)
        }
      })
      .prompt()?;

    let description = Text::new("Short public description of the strand:")
      .with_validator(not_empty)
      .with_validator(|text: &str| {
        if text.len() < 120 {
          Ok(Validation::Valid)
        } else {
          Ok(Validation::Invalid("Description must be less than 120 characters".into()))
        }
      })
      .prompt()?;

    let existing_keys = ctx.key_store.keynames()?;

    let keyname = if existing_keys.len() > 0 {
      let choice = Select::new(
        "Which key to sign the strand with:",
        existing_keys.iter().cloned().chain(std::iter::once("Generate new key".into())).collect(),
      )
        .prompt()?;

      if choice == "Generate new key" {
        None
      } else {
        Some(choice)
      }
    } else {
      None
    };

    let keyname = match keyname {
      None => {
        let mut keyname = None;

        while keyname == None {
          let input = Text::new("Name for the new key:").prompt()?;

          if existing_keys.contains(&input) {
            if Confirm::new("Key already exists. Overwrite?").prompt()? {
              keyname = Some(input);
            }
          } else {
            keyname = Some(input);
          }
        }

        let signer = RingSigner::generate_ed25519().map_err(|e| anyhow::anyhow!("Failed to generate key. {}", e))?;

        let keyname = keyname.unwrap();
        ctx.key_store.save_keypair(&keyname, signer.pkcs8())?;
        keyname
      },
      Some(keyname) => keyname,
    };

    let signer = RingSigner::new(SignatureAlgorithm::Ed25519, ctx.key_store.load_keypair(&keyname)?).map_err(|e| anyhow::anyhow!("Failed to load key. {}", e))?;
    let builder = twine_builder::TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .details(ipld!({
        "description": description,
      }))
      .done()?;

    store.save(strand.clone()).await?;

    dbg!(store.strands().await?.map_ok(|s| s.cid().to_string()).collect::<Vec<_>>().await);

    config.strands.replace(StrandRecord {
      cid: strand.cid().into(),
      sync: false,
      key: Some(keyname.clone()),
      name: Some(nickname.clone()),
    });
    config.save()?;

    log::info!("Strand \"{}\" created: {}", nickname, strand.cid());

    Ok(())
  }

}
