use clap::Parser;
use config::Config;
use directories::ProjectDirs;
use indicatif::MultiProgress;
use simplelog::{ConfigBuilder, TermLogger};
use anyhow::Result;

mod config;
mod cli;
mod selector;
mod stores;
mod prompt;
pub(crate) mod cid_str;

lazy_static::lazy_static! {
  pub(crate) static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("rs", "twine", "twine_cli")
    .expect("Could not determine local store path");
}

#[derive(Debug)]
pub(crate) struct Context {
  multi_progress: MultiProgress,
  cfg: Option<Config>,
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = cli::Cli::parse();
  let log_level = match (cli.quiet, cli.verbose) {
    (true, _) => log::LevelFilter::Off,
    (_, 0) => log::LevelFilter::Warn,
    (_, 1) => log::LevelFilter::Info,
    (_, 2) => log::LevelFilter::Debug,
    (_, _) => log::LevelFilter::Trace,
  };
  let config = {
    let mut c = ConfigBuilder::new();
    c
      .set_time_level(log::LevelFilter::Debug)
      .set_target_level(log::LevelFilter::Trace)
      .set_location_level(log::LevelFilter::Off)
      .set_max_level(log::LevelFilter::Debug);

    #[cfg(not(debug_assertions))]
    {
      c
        .add_filter_ignore_str("reqwest")
        .add_filter_ignore_str("sled")
        .add_filter_ignore_str("hyper_util")
        .add_filter_ignore_str("tokio_util");
    }

    c.build()
  };
  let mode = simplelog::TerminalMode::Mixed;
  let color_choice = simplelog::ColorChoice::Auto;
  let logger = TermLogger::new(log_level, config, mode, color_choice);

  let multi_progress = MultiProgress::new();
  indicatif_log_bridge::LogWrapper::new(
    multi_progress.clone(),
    logger
  ).try_init()?;

  let config = Config::load_local()?;
  let result = cli.run(
    Context {
      multi_progress: multi_progress.clone(),
      cfg: config,
    }
  ).await;

  if let Err(e) = result {
    log::error!("Error: {}", e);
    Err(e)
  } else {
    Ok(())
  }
}
