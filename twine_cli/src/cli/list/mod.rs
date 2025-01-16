use std::sync::Arc;
use clap::Parser;
use anyhow::Result;
use twine_car_store::CarStore;
use twine_core::{errors::ResolutionError, resolver::{unchecked_base::BaseResolver, Query, RangeQuery, Resolver, ResolverSetSeries}, twine::{Strand, Twine}, Cid, Ipld};
use futures::stream::{Stream, StreamExt, TryStreamExt};
use num_format::{ToFormattedString, SystemLocale};
use crate::selector::{Selector, parse_selector};

fn is_a_path(path: impl AsRef<str>) -> bool {
  std::path::Path::new(path.as_ref()).exists()
}

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

fn format_ipld(thing: &Ipld, depth: u8, locale: &SystemLocale) -> String {
  match thing {
    Ipld::String(s) => {
      s.to_string()
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

impl ListCommand {
  // list strands from resolver
  pub async fn run(&self, config: &crate::config::Config, _ctx: crate::Context) -> Result<()> {
    log::trace!("List: {:?}", self);

    let store = config.get_local_store()?;
    let resolver = if self.resolver.as_ref().map(is_a_path).unwrap_or(false) {
      // *.store.car files in current directory
      let car_files = std::fs::read_dir(self.resolver.as_ref().unwrap())?
        .filter_map(|entry| {
          let entry = entry.ok()?;
          let path = entry.path();
          if path.to_str()?.ends_with(".store.car") {
            Some(path)
          } else {
            None
          }
        })
        .collect::<Vec<_>>();

      let stores = futures::stream::iter(car_files)
        .map(|path| CarStore::new(path))
        .map_ok(|store| Box::new(store) as Box<dyn BaseResolver>)
        .try_collect::<Vec<_>>().await?;

      ResolverSetSeries::new(stores)
    } else {
      ResolverSetSeries::new(vec![Box::new(store), config.get_resolver(&self.resolver)?])
    };

    match &self.selector {
      Some(selector) => match selector {
        Selector::All => self.list_strands(&resolver).await?,
        Selector::Strand(cid) => self.list_strand(&cid, &resolver).await?,
        Selector::Query(query) => self.list_query(*query, &resolver).await?,
        Selector::RangeQuery(range) => self.list_range(*range, &resolver).await?,
      },
      None => self.list_strands(&resolver).await?,
    }

    Ok(())
  }

  async fn list_strand<R: Resolver>(&self, cid: &Cid, resolver: &R) -> Result<()> {
    log::trace!("Resolving cid {}", cid);
    let strand = resolver.resolve_strand(cid).await?.unpack();
    self.print_strand_stream(
      futures::stream::once(async { Ok(strand) }),
      resolver
    ).await?;
    Ok(())
  }

  async fn list_query<R: Resolver>(&self, query: Query, resolver: &R) -> Result<()> {
    log::trace!("Resolving query {}", query);
    let twine = resolver.resolve(query).await?.unpack();
    self.print_twine_stream(
      futures::stream::once(async { Ok(twine) })
    ).await?;
    Ok(())
  }

  async fn list_range<R: Resolver>(&self, range: RangeQuery, resolver: &R) -> Result<()> {
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

  async fn list_strands<R: Resolver>(&self, resolver: &R) -> Result<()> {
    log::trace!("Listing strands");
    let strands = resolver.strands().await?;
    self.print_strand_stream(strands, resolver).await?;
    Ok(())
  }

  async fn print_strand_stream<S: Stream<Item = Result<Arc<Strand>, ResolutionError>>, R: Resolver>(&self, strands: S, resolver: &R) -> Result<()> {
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
      .map(|s| async {
        let strand = s?;
        match resolver.resolve_latest(&strand).await {
          Ok(latest) => Ok((strand, Some(latest))),
          Err(ResolutionError::NotFound) => {
            log::debug!("No latest tixel for strand {}", strand.cid());
            Ok((strand, None))
          },
          Err(err) => {
            log::error!("{}", err);
            Err(err)
          }
        }
      })
      .buffered(2)
      .try_for_each(|(strand, maybe_latest)| async move {
        let cid = strand.cid();
        let latest_index = maybe_latest.as_ref().map(|l| l.index().to_formatted_string(locale));
        if self.inspect {
          let subspec = strand.subspec().map(|s| s.to_string()).unwrap_or_default();
          println!("{}", cid);
          println!("  Latest: {}", latest_index.unwrap_or("unknown".to_string()));
          if maybe_latest.is_some() {
            let latest = maybe_latest.unwrap();
            let byte_count = latest.bytes().len();
            println!(
              "  Estimated strand size (MB): {}",
              (latest.index() as usize * byte_count) / 1_000_000
            );
          }
          println!("  Subspec: {}", subspec);
          println!("  Key: {}", strand.key().alg);
          let details = format_ipld(strand.details(), self.depth, locale);
          println!("  Details: {}", indent::indent_all_by(2, details));
        } else {
          println!("{}", cid);
        }
        Ok(())
      }).await?;
    Ok(())
  }
}
