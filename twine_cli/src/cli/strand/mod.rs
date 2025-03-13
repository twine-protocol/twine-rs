use std::collections::HashSet;
use clap::Parser;
use indent::indent_all_by;
use twine_lib::ipld_core::serde::from_ipld;
use twine_lib::resolver::*;
use anyhow::Result;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PrintableDetails {
  name: Option<String>,
  description: Option<String>,
}

impl std::fmt::Display for PrintableDetails {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(name) = &self.name {
      writeln!(f, "Public Name: {}", name)?;
    }
    if let Some(description) = &self.description {
      writeln!(f, "Public Description: {}", description)?;
    }
    Ok(())
  }
}

#[derive(Debug, Parser)]
pub struct StrandCommand {
}

impl StrandCommand {
  pub async fn run(&self, config: &mut crate::config::Config, _ctx: crate::Context) -> Result<()> {
    let store = config.get_local_store()?;
    let owned = config.strands.iter()
      .filter(|s| s.key.is_some())
      .collect::<HashSet<_>>();

    // print
    for record in owned {
      let strand = store.resolve_strand(&record.cid).await?;
      println!("{}", record.cid);
      println!("  Name: {}", record.name.as_deref().unwrap_or("Unnamed"));
      println!("  Key: {}", record.key.as_ref().unwrap());
      println!("  Algorithm: {}", strand.key().alg);
      println!("  Sync enabled: {}", record.sync);
      let details: PrintableDetails = from_ipld(strand.details().clone())?;
      println!("  Details: \n{}", indent_all_by(4, details.to_string()));
    }

    Ok(())
  }
}
