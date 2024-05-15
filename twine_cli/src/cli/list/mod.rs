use std::{f32::consts::E, sync::Arc};

use clap::{Subcommand, Parser};
use anyhow::Result;
use twine_core::{errors::ResolutionError, resolver::{Query, RangeQuery, Resolver}, twine::{AnyTwine, Stitch, Strand, Twine}, Cid, Ipld};
use futures::stream::{Stream, StreamExt, TryStreamExt};
use num_format::{Locale, ToFormattedString, SystemLocale};

// use crate::poly_resolver::PolyResolver;

#[derive(Debug, Parser)]
pub struct ListCommand {
  /// Strand selector
  #[arg(value_parser = parse_selector)]
  selector: Option<Selector>,
  /// Use specified resolver (otherwise use default resolver)
  #[arg(short, long)]
  resolver: Option<String>,
  /// Show details
  #[arg(short, long)]
  inspect: bool,
  /// Output as JSON (ignores --inspect)
  #[arg(short, long)]
  json: bool,
  /// Recursion depth for inspect
  #[arg(short, long, default_value = "1")]
  depth: u8,
}

fn format_ipld(thing: Ipld, depth: u8, locale: &SystemLocale) -> String {
  match thing {
    Ipld::String(s) => {
      s
    },
    Ipld::Bool(b) => {
      b.to_string()
    },
    Ipld::Integer(i) => {
      i.to_formatted_string(locale)
    },
    Ipld::Float(f) => {
      format!("{:e}", f)
    },
    Ipld::Link(l) => {
      l.to_string()
    },
    Ipld::Bytes(b) => {
      // format Vec<u8> as hex string
      format!("{}", b.iter().fold(String::new(), |mut acc, byte| {
        acc.push_str(&format!("{:02x}", byte));
        acc
      }))
    },
    Ipld::List(items) => {
      if depth == 0 {
        "List(...)".to_string()
      } else {
        let mut string = String::new();
        for item in items {
          let item = format_ipld(item, depth - 1, locale);
          string.push_str(&format!("\n{}", item));
        }
        indent::indent_all_by(2, string)
      }
    },
    Ipld::Map(items) => {
      if depth == 0 {
        "Map(...)".to_string()
      } else {
        let mut string = String::new();
        for (key, value) in items {
          let value = format_ipld(value, depth - 1, locale);
          string.push_str(&format!("\n{}: {}", key, value));
        }
        indent::indent_all_by(2, string)
      }
    },
    Ipld::Null => {
      "null".to_string()
    },
  }
}

#[derive(Debug, Clone, Copy)]
enum Selector {
  Strand(Cid),
  Query(Query),
  RangeQuery(RangeQuery),
}

// expects format <cid>[:<index>?[:<lower_index>?]]
// ... could be <cid>:: (whole range),
// <cid>::<lower_index> (range from latest to lower_index)
// <cid>:<upper_index>: (range from upper_index to 0)
fn parse_selector(selector: &str) -> Result<Selector> {
  match selector.split(':').count() {
    1 => {
      let cid = Cid::try_from(selector)?;
      Ok(Selector::Strand(cid))
    },
    2 => {
      Ok(Selector::Query(selector.parse()?))
    },
    3 => {
      Ok(Selector::RangeQuery(selector.parse()?))
    },
    _ => Err(anyhow::anyhow!("Invalid selector format")),
  }
}


impl ListCommand {
  // list strands from resolver
  pub async fn run(&self, config: &crate::config::Config) -> Result<()> {
    log::trace!("List: {:?}", self);

    let resolver = match &self.resolver {
      Some(resolver) => config.resolvers.get(resolver).ok_or(anyhow::anyhow!("Resolver not found"))?,
      None => config.resolvers.get_default().ok_or(anyhow::anyhow!("No default resolver set. Please specify a resolver with -r"))?,
    }.as_resolver()?;

    let resolver = Resolver::new(&*resolver);

    match &self.selector {
      Some(selector) => match selector {
        Selector::Strand(cid) => self.list_strand(&cid, resolver).await?,
        Selector::Query(query) => self.list_query(*query, resolver).await?,
        Selector::RangeQuery(range) => self.list_range(*range, resolver).await?,
      }
      None => self.list_strands(resolver).await?,
    }

    Ok(())
  }

  async fn list_strand(&self, cid: &Cid, resolver: Resolver<'_>) -> Result<()> {
    log::trace!("Resolving cid {}", cid);
    let strand = resolver.resolve_strand(cid).await?;
    self.print_strand_stream(
      futures::stream::once(async { Ok(strand) }),
      resolver
    ).await?;
    Ok(())
  }

  async fn list_query(&self, query: Query, resolver: Resolver<'_>) -> Result<()> {
    log::trace!("Resolving query {}", query);
    let twine = resolver.resolve(query).await?;
    self.print_twine_stream(
      futures::stream::once(async { Ok(twine) })
    ).await?;
    Ok(())
  }

  async fn list_range(&self, range: RangeQuery, resolver: Resolver<'_>) -> Result<()> {
    log::trace!("Resolving range {}", range);
    let stream = resolver.resolve_range(range).await?;
    self.print_twine_stream(stream).await?;
    Ok(())
  }

  async fn print_twine_stream<S: Stream<Item = Result<Twine, ResolutionError>>>(&self, stream: S) -> Result<()> {
    if self.json {
      stream
        .inspect_err(|err| {
          log::error!("{}", err);
        })
        .try_for_each(|twine| async {
          let twine = twine;
          println!("{}", twine);
          Ok(())
        }).await?;
      return Ok(());
    }

    let locale = &SystemLocale::default().unwrap();
    stream
      .inspect_err(|err| {
        log::error!("{}", err);
      })
      .try_for_each(|twine| async {
        let twine = twine;
        let cid = twine.cid();
        let strand_cid = twine.strand_cid();
        let index = twine.index();
        if self.inspect {
          let subspec = twine.subspec().map(|s| s.to_string()).unwrap_or_default();
          let payload = format_ipld(twine.payload(), self.depth, locale);
          println!("{}", cid);
          println!("  Strand: {}", strand_cid);
          println!("  Index: {}", index);
          println!("  Subspec: {}", subspec);
          println!("  Payload: {}", indent::indent_all_by(2, payload));
        } else {
          println!("({}) {}:{}", index, strand_cid, cid);
        }
        Ok(())
      }).await?;
    Ok(())
  }

  async fn list_strands(&self, resolver: Resolver<'_>) -> Result<()> {
    log::trace!("Listing strands");
    let strands = resolver.strands().await?;
    self.print_strand_stream(strands, resolver).await?;
    Ok(())
  }

  async fn print_strand_stream<S: Stream<Item = Result<Arc<Strand>, ResolutionError>>>(&self, strands: S, resolver: Resolver<'_>) -> Result<()> {
    if self.json {
      strands
        .inspect_err(|err| {
          log::error!("{}", err);
        })
        .try_for_each(|twine| async {
          let twine = twine;
          println!("{}", twine);
          Ok(())
        }).await?;
      return Ok(());
    }

    let locale = &SystemLocale::default().unwrap();
    strands
      .inspect_ok(|strand| {
        log::trace!("Resolving latest for strand {}", strand.cid());
      })
      .map(|s| async { resolver.resolve_latest(s?).await })
      .buffered(2)
      .inspect_err(|err| {
        log::error!("{}", err);
      })
      .try_for_each(|latest| async move {
        let strand = latest.strand();
        let cid = strand.cid();
        let latest_index = latest.index().to_formatted_string(locale);
        if self.inspect {
          let subspec = strand.subspec().map(|s| s.to_string()).unwrap_or_default();
          println!("{}", cid);
          println!("  Latest: {}", latest_index);
          println!("  Subspec: {}", subspec);
          println!("  Key: {}", strand.key().key_type());
          let details = format_ipld(strand.details(), self.depth, locale);
          println!("  Details: {}", indent::indent_all_by(2, details));
        } else {
          println!("{} (latest: {})", cid, latest_index);
        }
        Ok(())
      }).await?;
    Ok(())
  }
}
