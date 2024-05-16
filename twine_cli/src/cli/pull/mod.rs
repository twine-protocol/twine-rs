use clap::Parser;
use anyhow::Result;
use twine_core::{store::Store, errors::ResolutionError, resolver::{Query, RangeQuery, Resolver}, twine::{AnyTwine, Stitch, Strand, Twine}, Cid, Ipld};
use futures::{stream::{Stream, StreamExt, TryStreamExt}, TryFutureExt};
use num_format::{ToFormattedString, SystemLocale};
use twine_sled_store::SledStore;
use crate::selector::{Selector, parse_selector};

#[derive(Debug, Parser)]
pub struct PullCommand {
  /// Strand selector
  #[arg(value_parser = parse_selector)]
  selector: Option<Selector>,
  /// Use specified resolver (otherwise use default resolver)
  #[arg(short, long)]
  resolver: Option<String>,
}

impl PullCommand {
  pub async fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    let resolver = config.get_resolver(&self.resolver)?;
    let store = config.get_local_store()?;

    match &self.selector {
      Some(selector) => match selector {
        Selector::Strand(cid) => self.pull(&store, &resolver, (cid, ..).into()).await?,
        Selector::Query(query) => self.pull_one(&store, &resolver, *query).await?,
        Selector::RangeQuery(range) => self.pull(&store, &resolver, *range).await?,
      },
      None => {
        let mut strands = resolver.strands().await?;
        while let Some(strand) = strands.next().await {
          let strand = strand?;
          let range = (strand.cid(), ..).into();
          self.pull(&store, &resolver, range).await?;
        }
      }
    };

    Ok(())
  }

  async fn pull<R: Resolver>(&self, store: &SledStore, resolver: &R, range: RangeQuery) -> Result<()> {
    log::info!("Pulling twines from strand: {}", range.strand_cid());
    let strand = resolver.resolve_strand(range.strand_cid()).await?;
    log::debug!("Saving strand: {}", strand.cid());
    store.save(strand).await?;
    let stream = resolver.resolve_range(range).await?
      .map(|twine| {
        let twine = twine.expect("Error resolving twine");
        log::debug!("Saving twine: ({}) {}", twine.index(), twine.cid());
        twine
      });
    store.save_stream(stream).await?;
    Ok(())
  }

  async fn pull_one<R: Resolver>(&self, store: &SledStore, resolver: &R, query: Query) -> Result<()> {
    let twine = resolver.resolve(query).await?;
    log::debug!("Saving strand: {}", twine.strand_cid());
    store.save(twine.strand()).await?;
    log::debug!("Saving twine: ({}) {}", twine.index(), twine.cid());
    store.save(twine).await?;
    Ok(())
  }
}
