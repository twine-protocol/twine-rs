use clap::Parser;

mod config;
mod cli;
// mod poly_resolver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = cli::Cli::parse();
  stderrlog::new()
    .verbosity(cli.verbose as usize)
    .quiet(cli.quiet)
    .init()?;
  let mut config = config::load_config()?;
  cli.run(&mut config).await?;

  config.save()?;
  Ok(())
}
