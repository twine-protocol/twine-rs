use clap::Parser;

mod config;
mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = cli::Cli::parse();
  simple_logger::init_with_level(match cli.verbose {
    0 => log::Level::Info,
    1 => log::Level::Debug,
    _ => log::Level::Trace,
  })?;
  let mut config = config::load_config()?;
  cli.run(&mut config);

  config.save()?;
  Ok(())
}
