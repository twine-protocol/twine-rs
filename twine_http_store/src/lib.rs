pub use reqwest;
pub mod v1;
pub mod v2;

pub async fn determine_version(uri: &str) -> Option<u8> {
  let client = reqwest::Client::new();
  let res = client.head(uri).send().await.ok()?;
  let headers = res.headers();
  let version = headers.get("X-Spool-Version")?;
  version.to_str().ok()?.parse().ok()
}
