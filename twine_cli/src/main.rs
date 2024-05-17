use std::env;

use clap::Parser;
use indicatif::MultiProgress;

mod config;
mod cli;
mod selector;

#[derive(Debug)]
pub(crate) struct Context {
  multi_progress: MultiProgress,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = cli::Cli::parse();
  // let mut logger = stderrlog::new();
  // logger
  //   .verbosity(cli.verbose as usize)
  //   .quiet(cli.quiet)
  //   .show_level(cli.verbose > 1)
  //   .modules(vec![
  //     module_path!(),
  //   ]);
  let logger = env_logger::Builder::new()
    .format_timestamp(None)
    .format_module_path(false)
    .filter_level(match (cli.quiet, cli.verbose) {
      (true, _) => log::LevelFilter::Off,
      (_, 0) => log::LevelFilter::Warn,
      (_, 1) => log::LevelFilter::Info,
      (_, 2) => log::LevelFilter::Debug,
      (_, _) => log::LevelFilter::Trace,
    })
    .format_indent(Some(2))
    .build();

  let multi_progress = MultiProgress::new();
  indicatif_log_bridge::LogWrapper::new(
    multi_progress.clone(),
    logger
  ).try_init()?;

  let mut config = config::load_config()?;
  cli.run(
    &mut config,
    Context { multi_progress: multi_progress.clone() }
  ).await?;

  config.save()?;
  multi_progress.clear().unwrap();
  Ok(())
}
