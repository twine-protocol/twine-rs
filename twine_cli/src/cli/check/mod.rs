use std::sync::Arc;
use clap::Parser;
use anyhow::Result;
use twine_core::{errors::ResolutionError, resolver::{Query, RangeQuery, Resolver}, twine::{Strand, Twine}, Cid, Ipld};
use futures::stream::{Stream, StreamExt, TryStreamExt};
use num_format::{ToFormattedString, SystemLocale};
use crate::{selector::{parse_selector, Selector}, stores::resolver_from_args};

#[derive(Debug, Parser)]
pub struct CheckCommand {
  /// Strand selector
  #[arg(value_parser = parse_selector)]
  selector: Option<Selector>,
  /// Use specified resolver (otherwise use default resolver)
  #[arg(short, long)]
  resolver: Option<String>,
}

impl CheckCommand {
  // list strands from resolver
  pub async fn run(&self, ctx: crate::Context) -> Result<()> {
    log::trace!("Check: {:?}", self);

    let resolver = resolver_from_args(&self.resolver, &ctx.cfg)?;

    match &self.selector {
      Some(selector) => match selector {
        Selector::All => self.verify_strands(&resolver).await?,
        Selector::Strand(cid) => self.verify_strand(&cid, &resolver).await?,
        Selector::Query(_query) => return Err(anyhow::anyhow!("Specify a range or strand")),
        Selector::RangeQuery(range) => self.verify_range(*range, &resolver).await?,
      },
      None => self.verify_strands(&resolver).await?,
    }

    Ok(())
  }

  async fn verify_strand<R: Resolver>(&self, cid: &Cid, resolver: &R) -> Result<()> {
    let strand = resolver.resolve_strand(cid).await?.unpack();
    self.verify_range((strand, -1..0).into(), resolver).await?;
    Ok(())
  }

  async fn verify_range<R: Resolver>(&self, range: RangeQuery, resolver: &R) -> Result<()> {
    log::trace!("Checking range {}", range);
    let range = range.try_to_absolute(resolver).await?.ok_or_else(|| anyhow::anyhow!("Range empty"))?;
    if !range.is_decreasing() {
      return Err(anyhow::anyhow!("Range must be decreasing"));
    }
    let stream = resolver.resolve_range(range).await?;
    stream.map_err(|e| e.into()).try_fold(None, |upper: Option<Twine>, twine| async {
      if let Some(upper) = upper {
        let expected = match upper.previous() {
          Some(prev) => prev,
          None => {
            if upper.index() == 0 {
              return Ok(Some(twine));
            } else {
              return Err(anyhow::anyhow!("Twine {} (index: {}) has no previous", upper.cid(), upper.index()));
            }
          }
        };
        if expected != twine {
          return Err(anyhow::anyhow!("Chain broken at {}, index: {}", twine.cid(), twine.index()));
        }
      }
      Ok(Some(twine))
    }).await?;
    Ok(())
  }

  async fn verify_strands<R: Resolver>(&self, resolver: &R) -> Result<()> {
    log::trace!("Checking all strands");
    let strands = resolver.strands().await?;
    strands.map_err(|e| anyhow::anyhow!(e)).try_for_each(|strand| async move {
      let cid = strand.cid();
      let range = RangeQuery::from((cid, -1..0));
      self.verify_range(range, resolver).await?;
      Ok(())
    }).await?;
    Ok(())
  }
}
