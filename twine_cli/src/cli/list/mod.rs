use clap::{Subcommand, Parser};
use anyhow::Result;
use twine_core::{resolver::Resolver, Ipld};
use futures::stream::{Stream, StreamExt, TryStreamExt};
use num_format::{Locale, ToFormattedString, SystemLocale};

#[derive(Debug, Parser)]
pub struct ListCommand {
  /// Use specified resolver (otherwise use default resolver)
  #[arg(short, long)]
  pub resolver: Option<String>,
  /// Show details
  #[arg(short, long)]
  pub details: bool,
  /// Output as JSON (ignores --details)
  #[arg(short, long)]
  pub json: bool,
}

fn get_details(details: Ipld) -> Option<String> {
  let locale = SystemLocale::default().unwrap();
  match details {
    Ipld::Map(m) => {
      let mut s = String::new();
      for (k, v) in m {
        let v = match v {
          Ipld::String(s) => {
            s
          },
          Ipld::Bool(b) => {
            b.to_string()
          },
          Ipld::Integer(i) => {
            i.to_formatted_string(&locale)
          },
          Ipld::Float(f) => {
            f.to_string()
          },
          Ipld::Link(l) => {
            l.to_string()
          },
          _ => "(...)".to_string(),
        };
        s.push_str(&format!("    {}: {}\n", k, v));
      }
      Some(s)
    },
    _ => None,
  }
}

impl ListCommand {
  // list strands from resolver
  pub async fn run(&self, config: &crate::config::Config) -> Result<()> {
    let locale = &SystemLocale::default().unwrap();
    let resolver = match &self.resolver {
      Some(resolver) => config.resolvers.get(resolver).ok_or(anyhow::anyhow!("Resolver not found"))?,
      None => config.resolvers.get_default().ok_or(anyhow::anyhow!("No default resolver set. Please specify a resolver with -r"))?,
    };
    let resolver = resolver.as_resolver()?;
    let strands = resolver.strands().await?;

    if self.json {
      strands.try_for_each(|strand| async {
        let strand = strand;
        println!("{}", strand);
        Ok(())
      }).await?;
      return Ok(());
    }

    strands
      .map(|s| async { resolver.resolve_latest(s?).await })
      .buffered(2)
      .try_for_each(|latest| async move {
        let strand = latest.strand();
        let cid = strand.cid();
        let latest_index = latest.index().to_formatted_string(locale);
        if self.details {
          let subspec = strand.subspec().map(|s| s.to_string()).unwrap_or_default();
          let details = get_details(strand.details());
          println!("{}", cid);
          println!("  Latest: {}", latest_index);
          println!("  Subspec: {}", subspec);
          println!("  Details:");
          if let Some(details) = details {
            println!("{}", details);
          } else {
            println!("    (no details)");
          }
        } else {
          println!("{} (latest: {})", cid, latest_index);
        }
        Ok(())
      }).await?;
    Ok(())
  }
}
