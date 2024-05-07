use async_trait::async_trait;
use futures::{Stream, TryStreamExt};
use reqwest::{header::{ACCEPT, CONTENT_TYPE}, StatusCode, Url};
use rs_car::car_read_all;
use std::{pin::Pin, sync::Arc};
use std::time::Duration;
use twine_core::{twine::*, twine::TwineBlock, errors::*, as_cid::AsCid, store::Store, resolver::RangeQuery, Cid, resolver::AbsoluteRange};
use twine_core::resolver::Resolver;
use std::error::Error;

pub use reqwest;

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResolverOptions {
  pub url: Url,
  pub timeout: Duration,
  pub buffer_size: usize,
}

impl Default for HttpResolverOptions {
  fn default() -> Self {
    Self {
      url: "http://localhost:8080".parse().unwrap(),
      timeout: Duration::from_secs(30),
      buffer_size: 4,
    }
  }
}

impl HttpResolverOptions {
  pub fn url(mut self, url: &str) -> Self {
    self.url = format!("{}/", url).parse().expect("Invalid URL");
    self
  }

  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  pub fn buffer_size(mut self, buffer_size: usize) -> Self {
    self.buffer_size = buffer_size;
    self
  }
}

#[derive(Debug, Clone)]
pub struct HttpResolver {
  client: reqwest::Client,
  pub options: HttpResolverOptions,
}

impl Default for HttpResolver {
  fn default() -> Self {
    Self::new(
      reqwest::Client::new(),
      HttpResolverOptions::default()
    )
  }
}

impl HttpResolver {
  pub fn new(client: reqwest::Client, options: HttpResolverOptions) -> Self {
    Self {
      client,
      options,
    }
  }

  fn req(&self, path: &str) -> reqwest::RequestBuilder {
    self.client.get(self.options.url.join(&path).expect("Invalid path"))
      .header(ACCEPT, "application/vnd.ipld.car, application/json;q=0.5")
      .timeout(self.options.timeout)
  }

  fn send(&self, path: &str) -> reqwest::RequestBuilder {
    self.client.post(self.options.url.join(&path).expect("Invalid path"))
      .header(CONTENT_TYPE, "application/vnd.ipld.car")
      .timeout(self.options.timeout)
  }

  async fn parse(&self, response: reqwest::Response) -> Result<AnyTwine, ResolutionError> {
    match response.error_for_status_ref() {
      Ok(_) => {},
      Err(e) => {
        if let Some(StatusCode::NOT_FOUND) = e.status() {
          return Err(ResolutionError::NotFound);
        }
        return Err(ResolutionError::Fetch(e.to_string()));
      },
    }
    let tp = response.headers().get(CONTENT_TYPE).map(|h| h.to_str().unwrap_or("")).unwrap_or("");
    match tp {
      "application/vnd.ipld.car"|"application/octet-stream" => {
        let bytes = response.bytes().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let (blocks, header) = car_read_all(&mut bytes.as_ref(), false).await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let root = header.roots.first().ok_or(ResolutionError::Fetch("No roots found".to_string()))?;
        let (cid, bytes) = blocks.iter().find(|(cid, _)| cid == root).ok_or(ResolutionError::Fetch("Root not found".to_string()))?;
        let twine = AnyTwine::from_block(*cid, bytes).map_err(|e| ResolutionError::Invalid(e))?;
        Ok(AnyTwine::from(twine))
      },
      _ => {
        let json = response.text().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let twine = AnyTwine::from_dag_json(&json).map_err(|e| ResolutionError::Invalid(e))?;
        Ok(twine)
      },
    }
  }

  async fn parse_expect(&self, expected: &Cid, response: reqwest::Response) -> Result<AnyTwine, ResolutionError> {
    let twine = self.parse(response).await?;
    if twine.cid() == *expected {
      Ok(twine)
    } else {
      Err(ResolutionError::Invalid(VerificationError::CidMismatch { expected: expected.to_string(), actual: twine.cid().to_string() }))
    }
  }

  async fn get_tixel<T: AsCid + Send>(&self, strand: T, path: &str) -> Result<Twine, ResolutionError> {
    let strand = self.resolve_strand(strand.as_cid());
    let response = self.req(&path).send();
    let (strand, response) = futures::future::join(strand, response).await;
    let strand = strand?;
    let response = response.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    let tixel = self.parse(response).await?.try_into()?;
    let twine = Twine::try_new_from_shared(strand, tixel)?;
    Ok(twine)
  }

  async fn fetch_tixel_range(&self, range: AbsoluteRange) -> Result<reqwest::Response, ResolutionError> {
    let path = format!("chains/{}/pulses/{}-{}", range.strand.as_cid(), range.upper, range.lower);
    let response = self.req(&path).send().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    match response.error_for_status_ref() {
      Ok(_) => {},
      Err(e) => {
        if let Some(StatusCode::NOT_FOUND) = e.status() {
          return Err(ResolutionError::NotFound);
        }
        return Err(ResolutionError::Fetch(e.to_string()));
      },
    }
    Ok(response)
  }

  async fn parse_collection_response(&self, response: reqwest::Response) -> Result<impl Stream<Item = Result<AnyTwine, ResolutionError>>, ResolutionError> {
    let tp = response.headers().get(CONTENT_TYPE).map(|h| h.to_str().unwrap_or("")).unwrap_or("");
    use futures::stream::StreamExt;
    match tp {
      "application/vnd.ipld.car"|"application/octet-stream" => {
        let bytes = response.bytes().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        // TODO: I'd love to use CarReader and stream this but it only borrows,
        // so I can't give it ownership of the bytes
        let (blocks, _) = car_read_all(&mut bytes.as_ref(), false).await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let stream = futures::stream::iter(blocks)
          .then(|(cid, bytes)| async move {
            AnyTwine::from_block(cid, bytes).map_err(|e| ResolutionError::Invalid(e))
          });
        Ok(stream.boxed())
      },
      _ => {
        let json = response.text().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let twines = AnyTwine::from_dag_json_array(json).map_err(|e| ResolutionError::Invalid(e))?;
        let stream = futures::stream::iter(twines.into_iter()).map(|t| Ok(t.clone()));
        Ok(stream.boxed())
      },
    }
  }
}

#[async_trait]
impl Resolver for HttpResolver {
  async fn resolve_cid<'a, C: AsCid + Send>(&'a self, cid: C) -> Result<AnyTwine, ResolutionError> {
    let cid = cid.as_cid();
    let path = format!("cid/{}", cid);
    let response = self.req(&path).send().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(self.parse_expect(cid, response).await?)
  }

  async fn strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let response = self.req("chains").send().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    use futures::stream::StreamExt;
    let stream = self.parse_collection_response(response).await?;
    let stream = stream.map(|t| {
      let strand = Arc::<Strand>::try_from(t?)?;
      Ok(strand)
    });
    Ok(stream.boxed())
  }

  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    let cid = strand.as_cid();
    let path = format!("chains/{}", cid);
    let response = self.req(&path).send().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(self.parse_expect(cid, response).await?.try_into()?)
  }

  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
    let path = format!("chains/{}/pulses/{}", strand.as_cid(), index);
    let twine = self.get_tixel(strand, &path).await?;
    if twine.index() != index {
      return Err(ResolutionError::BadData(format!("Expected index {}, found {}", index, twine.index())));
    }
    Ok(twine)
  }

  async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
    let path = format!("chains/{}/pulses/latest", strand.as_cid());
    let twine = self.get_tixel(strand, &path).await?;
    Ok(twine)
  }

  async fn resolve_range<R: Into<RangeQuery> + Send>(&self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + '_>>, ResolutionError> {
    let range = range.into();
    use futures::stream::StreamExt;
    let strand = self.resolve_strand(range.strand_cid()).await?;
    let stream = range.to_batch_stream(self, 100)
      .map(|range| async {
        let response = self.fetch_tixel_range(range?).await;
        self.parse_collection_response(response?).await
      })
      .buffered(self.options.buffer_size)
      .try_flatten()
      .then(move |t| {
        let strand = strand.clone();
        async move {
          let tixel = Arc::<Tixel>::try_from(t?)?;
          Ok(Twine::try_new_from_shared(strand, tixel)?)
        }
      });
    Ok(stream.boxed())
  }
}

#[async_trait]
impl Store for HttpResolver {
  async fn save<T: Into<AnyTwine> + Send + Sync>(&self, twine: T) -> Result<(), Box<dyn Error>> {
    let twine = twine.into();
    let strand_cid = twine.strand_cid();
    let root = twine.cid();
    use twine_core::car::to_car_stream;
    use futures::stream::StreamExt;
    let data = to_car_stream(futures::stream::iter(vec![twine]), vec![root]);
    let vec = data.collect::<Vec<_>>().await.concat();
    let path = format!("chains/{}/pulses", strand_cid);
    self.send(&path)
      .body(vec)
      .send()
      .await?;
    Ok(())
  }

  async fn save_many<I: Into<AnyTwine> + Send + Sync, S: Iterator<Item = I> + Send + Sync, T: IntoIterator<Item = I, IntoIter = S> + Send + Sync>(&self, twines: T) -> Result<(), Box<dyn Error>> {
    let iter = twines.into_iter();
    let twines = iter.map(|t| t.into());
    use twine_core::car::to_car_stream;
    use futures::stream::StreamExt;
    let twines: Vec<AnyTwine> = twines.collect();
    let (cid, strand_cid) = match twines.first() {
      Some(t) => (t.cid(), t.strand_cid()),
      None => {
        return Ok(());
      }
    };
    let roots = vec![cid];
    let data = to_car_stream(futures::stream::iter(twines), roots);
    let vec = data.collect::<Vec<_>>().await.concat();
    let path = format!("chains/{}/pulses", strand_cid);
    self.send(&path)
      .body(vec)
      .send()
      .await?;
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send + Sync, T: Stream<Item = I> + Send + Sync + Unpin>(&self, twines: T) -> Result<(), Box<dyn Error>> {
    use futures::stream::StreamExt;
    self.save_many(twines.collect::<Vec<_>>().await).await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send + Sync>(&self, cid: C) -> Result<(), Box<dyn Error>> {
    self.client.delete(
      self.options.url.join(&format!("cid/{}", cid.as_cid())).expect("Invalid path")
    )
      .send()
      .await?;
    Ok(())
  }
}
