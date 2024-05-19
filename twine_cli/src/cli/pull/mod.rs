use indicatif::{ProgressBar, ProgressStyle};
use clap::Parser;
use anyhow::Result;
use twine_core::{errors::ResolutionError, resolver::{AbsoluteRange, Query, RangeQuery, Resolver}, store::Store};
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
  pub async fn run(&self, config: &mut crate::config::Config, mut ctx: crate::Context) -> Result<()> {
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
      match self.pull(&store, &resolver, range, &mut ctx).await {
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

  async fn pull<R: Resolver>(&self, store: &SledStore, resolver: &R, range: RangeQuery, ctx: &mut crate::Context) -> Result<()> {
    log::info!("Pulling twines from strand: {}", range.strand_cid());
    let strand = resolver.resolve_strand(range.strand_cid()).await?;
    log::debug!("Saving strand: {}", strand.cid());
    store.save(strand).await?;

    let range = range.try_to_absolute(resolver).await?;
    // only allow increasing ranges
    if range.is_decreasing() {
      return Err(anyhow::anyhow!("Cannot pull decreasing range"));
    }

    let range = if self.force { range } else {
      // first figure out what we have locally
      match store.resolve_latest(range.strand_cid()).await {
        Ok(twine) => {
          let latest_index = twine.index();
          // if we have latest, then assume we're done
          if latest_index >= range.upper() {
            return Ok(());
          }

          // if latest is below lower, then error
          if latest_index < range.lower() {
            return Err(anyhow::anyhow!("Local twine index is lower than requested range"));
          }

          // otherwise start from latest
          AbsoluteRange::new(
            *range.strand_cid(),
            latest_index,
            range.end
          )
        },
        Err(ResolutionError::NotFound) if range.lower() == 0 => {
          range
        },
        Err(e) => {
          return Err(e.into());
        }
      }
    };

    let lower = range.lower();
    let total_size = range.len();
    let pb = ctx.multi_progress.add(ProgressBar::new(total_size));
    pb.set_style(
      ProgressStyle::with_template( "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg} (eta: {eta})")
        .unwrap()
        .progress_chars("##-")
    );

    use futures::future::ready;
    let stream = resolver.resolve_range(range).await?
      .take_while(|res| {
        if res.is_ok() {
          ready(true)
        } else {
          pb.finish_with_message("Error");
          ready(false)
        }
      })
      .map(|res| {
        let twine = res.unwrap();
        pb.set_position(twine.index() - lower);
        pb.set_message(format!("index: {}", twine.index()));
        twine
      });

    match store.save_stream(stream).await {
      Ok(_) => pb.finish_with_message("Finished"),
      Err(e) => {
        pb.finish_with_message("Error");
        return Err(e.into());
      }
    };
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
