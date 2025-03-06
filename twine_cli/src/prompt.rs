use anyhow::Result;
use inquire::{validator::Validation, Confirm, Text};

pub fn not_empty(
  text: &str,
) -> std::result::Result<Validation, Box<dyn std::error::Error + Send + Sync>> {
  if text.is_empty() {
    Ok(Validation::Invalid("This field cannot be empty".into()))
  } else {
    Ok(Validation::Valid)
  }
}

#[allow(dead_code)]
pub fn only_simple_chars(
  text: &str,
) -> std::result::Result<Validation, Box<dyn std::error::Error + Send + Sync>> {
  if text
    .chars()
    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
  {
    Ok(Validation::Valid)
  } else {
    Ok(Validation::Invalid(
      "Only alphanumeric characters, dashes, and underscores are allowed".into(),
    ))
  }
}

pub fn prompt_for_filename(text: &str, default: &str) -> Result<String> {
  loop {
    let filename = Text::new(text)
      .with_validator(not_empty)
      .with_default(default)
      .prompt()?;

    // resolve the tilde
    let filename = shellexpand::tilde(&filename).into_owned();
    if std::path::Path::new(&filename).exists() {
      let overwrite = Confirm::new("File already exists. Overwrite?").prompt()?;
      if !overwrite {
        continue;
      }
    }
    return Ok(filename);
  }
}

pub fn prompt_for_directory(text: &str, default: &str) -> Result<String> {
  loop {
    let directory = Text::new(text)
      .with_validator(not_empty)
      .with_default(default)
      .prompt()?;

    // resolve the tilde
    let directory = shellexpand::tilde(&directory).into_owned();
    if std::path::Path::new(&directory).exists() {
      let overwrite = Confirm::new("Directory already exists. Use anyway?").prompt()?;
      if !overwrite {
        continue;
      }
    }
    return Ok(directory);
  }
}
