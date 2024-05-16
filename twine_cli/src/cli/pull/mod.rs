use std::io::IsTerminal;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use clap::Parser;
use anyhow::Result;
use twine_core::{resolver::{AbsoluteRange, Query, RangeQuery, Resolver}, store::Store};
use futures::{stream::StreamExt, TryStreamExt};
use twine_sled_store::SledStore;
use crate::selector::{Selector, parse_selector};

#[derive(Debug, Parser)]
pub struct PullCommand {
  /// Strand selector. If not provided, strands being synched will be pulled.
  #[arg(value_parser = parse_selector)]
  selector: Option<Selector>,
  /// Use specified resolver (otherwise use default resolver)
  #[arg(short, long)]
  resolver: Option<String>,
  /// Force full re-pull
  #[arg(short, long)]
  force: bool,
}

impl PullCommand {
  pub async fn run(&self, config: &mut crate::config::Config) -> Result<()> {
    let resolver = config.get_resolver(&self.resolver)?;
    let store = config.get_local_store()?;

    let ranges = match &self.selector {
      Some(selector) => match selector {
        Selector::Query(query) => {
          self.pull_one(&store, &resolver, *query).await?;
          log::info!("Finished pulling strand: {}", query.strand_cid());
          return Ok(());
        },
        Selector::Strand(cid) => vec![(cid, ..).into()],
        Selector::RangeQuery(range) => vec![*range],
        Selector::All => {
          resolver.strands().await?
            .map_ok(|strand| RangeQuery::from((strand.cid(), ..)))
            .try_collect()
            .await?
        }
      },
      None => {
        config.sync_strands.iter().map(|cid| (cid, ..).into()).collect::<Vec<RangeQuery>>()
      }
    };
    let mut errors = vec![];
    for range in ranges {
      match self.pull(&store, &resolver, range).await {
        Ok(_) => log::info!("Finished pulling strand: {}", range.strand_cid()),
        Err(e) => {
          log::error!("Error pulling strand: {}", e);
          errors.push(e);
        },
      }
    }
    if !errors.is_empty() {
      log::warn!("Errors occurred while pulling strands");
      for e in errors {
        log::error!("{}", e);
      }
      return Err(anyhow::anyhow!("Errors occurred while pulling strands"));
    }
    log::info!("Pull complete");
    Ok(())
  }

  async fn pull<R: Resolver>(&self, store: &SledStore, resolver: &R, range: RangeQuery) -> Result<()> {
    log::info!("Pulling twines from strand: {}", range.strand_cid());
    let strand = resolver.resolve_strand(range.strand_cid()).await?;
    log::debug!("Saving strand: {}", strand.cid());
    store.save(strand).await?;

    let range = if self.force { range } else {
      let range = range.try_to_absolute(resolver).await?;
      // first figure out what we have locally
      let has_lower = store.has((range.strand_cid(), range.lower)).await?;

      if has_lower {
        // then assume we have everything from lower to latest
        let latest = store.resolve_latest(range.strand_cid()).await?;
        AbsoluteRange::new(
          *range.strand_cid(),
          range.upper,
          latest.index()
        ).into()
      } else {
        range.into()
      }
    };

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
