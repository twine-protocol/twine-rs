use crate::{
  selector::{parse_selector, Selector},
  stores::{parse_store, resolver_from_args, AnyStore},
};
use anyhow::Result;
use clap::Parser;
use futures::{stream::StreamExt, TryStreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use twine_core::{
  errors::ResolutionError,
  resolver::{AbsoluteRange, RangeQuery, Resolver, SingleQuery},
};

fn last_chars(s: &str, n: usize) -> &str {
  let start = s.len().saturating_sub(n);
  &s[start..]
}

lazy_static::lazy_static! {
  static ref CTRLC : Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Debug, Parser)]
pub struct SyncCommand {
  /// Strand selector. If not provided, strands being synched will be pulled.
  #[arg(value_parser = parse_selector)]
  selector: Selector,
  /// Destination store (eg: pickle://./store.db)
  #[arg(value_parser = parse_store)]
  store: AnyStore,
  /// Use specified resolver (otherwise use default resolver)
  #[arg(short, long)]
  resolver: Option<String>,
  /// Force full re-pull
  #[arg(short, long)]
  force: bool,
  /// Number of parallel pulls
  #[arg(short, long, default_value = "1")]
  parallel: usize,
}

impl SyncCommand {
  pub async fn run(&self, ctx: crate::Context) -> Result<()> {
    let resolver = resolver_from_args(&self.resolver, &ctx.cfg)?;
    let store = &self.store;

    let ranges = match &self.selector {
      Selector::SingleQuery(query) => {
        self.pull_one(&store, &resolver, *query).await?;
        log::info!("Finished syncing strand: {}", query.strand_cid());
        return Ok(());
      }
      Selector::Strand(cid) => vec![(cid, ..).into()],
      Selector::RangeQuery(range) => vec![*range],
      Selector::All => {
        resolver
          .strands()
          .await?
          .map_ok(|strand| RangeQuery::from((strand.cid(), ..)))
          .try_collect()
          .await?
      }
    };

    ctrlc::set_handler(|| {
      let mut ctrlc = CTRLC.lock().unwrap();
      log::warn!("Ctrl-C detected, stopping...");
      *ctrlc = true;
    })
    .expect("Error setting Ctrl-C handler");

    let bar = ProgressBar::new(ranges.len() as u64);

    use futures::stream::iter;
    let tasks: Vec<_> = iter(ranges)
      .map(|r| {
        let f = r.try_to_absolute(&resolver);
        async {
          let range = f.await?;
          if range.is_none() {
            return Err(anyhow::anyhow!("Range is empty"));
          }
          let range = range.unwrap();
          // only allow increasing ranges
          if range.is_decreasing() {
            return Err(anyhow::anyhow!("Cannot pull decreasing range"));
          }
          if self.force { Ok(range) } else {
            // first figure out what we have locally
            match store.resolve_latest(range.strand_cid()).await {
              Ok(twine) => {
                let latest_index = twine.index();
                // if we have latest, then assume we're done
                log::debug!("Latest index for strand {}: {}", range.strand_cid(), latest_index);
                if latest_index >= range.upper() {
                  return Ok(AbsoluteRange::new(
                    *range.strand_cid(),
                    range.end,
                    range.end
                  ));
                }

                // if latest is below lower, then error
                if latest_index < range.lower() {
                  return Err(anyhow::anyhow!("Local twine index is lower than requested range"));
                }

                // otherwise start from latest
                Ok(AbsoluteRange::new(
                  *range.strand_cid(),
                  latest_index,
                  range.end
                ))
              },
              Err(ResolutionError::NotFound) if range.lower() == 0 => {
                log::debug!("No twines pulled yet for strand {}", range.strand_cid());
                Ok(range)
              },
              Err(e) => {
                return Err(e.into());
              }
            }
          }
        }
      })
      .buffered(self.parallel)
      .map_ok(|r| {
        let pb = ctx.multi_progress.add(
          ProgressBar::new(r.upper()).with_message(format!("Pulling strand: {}", r.strand_cid()))
        );
        pb.set_style(
          ProgressStyle::with_template( "[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} {msg} (eta: {eta})")
            .unwrap()
            .progress_chars("=> ")
        );
        pb.set_position(0);
        pb.set_message("pending");
        (r, pb)
      })
      .try_collect().await?;

    let bar = ctx.multi_progress.add(bar);
    bar.set_style(
      ProgressStyle::with_template("{spinner} {msg} {pos:>7} of {len:7}")
        .unwrap()
        .progress_chars("=> "),
    );
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_message("Pulling...");

    let results: Vec<_> = iter(tasks)
      .map(|(r, pb)| self.pull(&store, &resolver, r, pb))
      .buffer_unordered(self.parallel)
      .inspect_err(|e| {
        ctx
          .multi_progress
          .println(format!("Error: {}", e))
          .unwrap_or_else(|e| log::error!("{}", e))
      })
      .inspect(|_| bar.inc(1))
      .collect()
      .await;

    let errors = results
      .into_iter()
      .filter_map(Result::err)
      .collect::<Vec<_>>();
    if !errors.is_empty() {
      log::warn!("Errors occurred while syncing strands");
      for e in errors {
        log::error!("{}", e);
      }
      return Err(anyhow::anyhow!("Errors occurred while syncing strands"));
    } else {
      log::debug!("Pull complete");
      bar.finish_with_message("Pull complete");
    }
    Ok(())
  }

  async fn pull<R: Resolver>(
    &self,
    store: &AnyStore,
    resolver: &R,
    range: AbsoluteRange,
    pb: ProgressBar,
  ) -> Result<()> {
    log::debug!("Pulling twines from strand: {}", range.strand_cid());
    let strand = resolver.resolve_strand(range.strand_cid()).await?.unpack();
    log::debug!("Saving strand: {}", strand.cid());
    store.save(strand).await?;

    pb.set_position(range.start);
    pb.reset_elapsed();
    pb.reset_eta();
    pb.enable_steady_tick(Duration::from_millis(300));
    pb.set_message(format!(
      "syncing (...{})",
      last_chars(&range.strand_cid().to_string(), 5)
    ));

    use futures::future::ready;
    let mut error = None;
    let stream = resolver
      .resolve_range(range)
      .await?
      .take_while(|res| {
        if CTRLC.lock().unwrap().clone() {
          return ready(false);
        }
        if res.is_ok() {
          ready(true)
        } else {
          error = res.as_ref().err().map(|e| e.to_string());
          ready(false)
        }
      })
      .map(|res| {
        let twine = res.unwrap();
        pb.set_position(twine.index());
        // pb.set_message(format!("remaining: {}", total_size - twine.index()));
        twine
      });

    match store.save_stream(stream).await {
      Ok(_) => {
        if let Some(err) = error {
          pb.abandon_with_message("Error!");
          Err(anyhow::anyhow!(
            "While syncing {}: {}",
            range.strand_cid(),
            err
          ))
        } else {
          if CTRLC.lock().unwrap().clone() {
            pb.abandon_with_message("Aborted!");
            Ok(())
          } else {
            pb.finish_with_message("Done!");
            Ok(())
          }
        }
      }
      Err(e) => {
        pb.abandon_with_message(format!("While syncing {}: {}", range.strand_cid(), e));
        Err(e.into())
      }
    }
  }

  async fn pull_one<R: Resolver>(
    &self,
    store: &AnyStore,
    resolver: &R,
    query: SingleQuery,
  ) -> Result<()> {
    let twine = resolver.resolve(query).await?.unpack();
    log::debug!("Saving strand: {}", twine.strand_cid());
    store.save(twine.strand().clone()).await?;
    log::debug!("Saving twine: ({}) {}", twine.index(), twine.cid());
    store.save(twine).await?;
    Ok(())
  }
}
