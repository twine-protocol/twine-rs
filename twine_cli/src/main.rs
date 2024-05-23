use clap::Parser;
use indicatif::MultiProgress;
use simplelog::{ConfigBuilder, TermLogger};

mod config;
mod cli;
mod selector;
pub(crate) mod cid_str;

#[derive(Debug)]
pub(crate) struct Context {
  multi_progress: MultiProgress,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = cli::Cli::parse();
  let log_level = match (cli.quiet, cli.verbose) {
    (true, _) => log::LevelFilter::Off,
    (_, 0) => log::LevelFilter::Warn,
    (_, 1) => log::LevelFilter::Info,
    (_, 2) => log::LevelFilter::Debug,
    (_, _) => log::LevelFilter::Trace,
  };
  let config = ConfigBuilder::new()
    .set_time_level(log::LevelFilter::Debug)
    .set_target_level(log::LevelFilter::Trace)
    .set_location_level(log::LevelFilter::Off)
    .set_max_level(log::LevelFilter::Debug)
    .add_filter_ignore_str("reqwest")
    .add_filter_ignore_str("sled")
    .add_filter_ignore_str("hyper_util")
    .add_filter_ignore_str("tokio_util")
    .build();
  let mode = simplelog::TerminalMode::Mixed;
  let color_choice = simplelog::ColorChoice::Auto;
  let logger = TermLogger::new(log_level, config, mode, color_choice);

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
  Ok(())
}
