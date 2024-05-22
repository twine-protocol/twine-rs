use async_trait::async_trait;
use futures::{Stream, TryStreamExt};
use reqwest::{header::{ACCEPT, CONTENT_TYPE}, StatusCode, Url};
use fvm_ipld_car::CarReader;
use std::{pin::Pin, sync::Arc};
use std::time::Duration;
use twine_core::{twine::*, twine::TwineBlock, errors::*, as_cid::AsCid, store::Store, Cid, resolver::AbsoluteRange};
use twine_core::resolver::BaseResolver;

pub use reqwest;

#[derive(Debug, Clone, PartialEq)]
pub struct HttpStoreOptions {
  pub url: Url,
  pub timeout: Duration,
  pub concurency: usize,
}

impl Default for HttpStoreOptions {
  fn default() -> Self {
    Self {
      url: "http://localhost:8080".parse().unwrap(),
      timeout: Duration::from_secs(30),
      concurency: 4,
    }
  }
}

impl HttpStoreOptions {
  pub fn url(mut self, url: &str) -> Self {
    self.url = format!("{}/", url).parse().expect("Invalid URL");
    self
  }

  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  pub fn concurency(mut self, concurency: usize) -> Self {
    self.concurency = concurency;
    self
  }
}

#[derive(Debug, Clone)]
pub struct HttpStore {
  client: reqwest::Client,
  pub options: HttpStoreOptions,
}

impl Default for HttpStore {
  fn default() -> Self {
    Self::new(
      reqwest::Client::new(),
      HttpStoreOptions::default()
    )
  }
}

fn handle_save_result(res: Result<reqwest::Response, ResolutionError>) -> Result<(), StoreError> {
  match res {
    Ok(_) => Ok::<(), StoreError>(()),
    Err(e) => {
      match e {
        ResolutionError::Fetch(e) => Err(StoreError::Saving(e)),
        ResolutionError::NotFound => Err(StoreError::Saving("Not found".to_string())),
        ResolutionError::Invalid(e) => Err(StoreError::Invalid(e)),
        ResolutionError::BadData(e) => Err(StoreError::Saving(e)),
      }
    },
  }
}

impl HttpStore {
  pub fn new(client: reqwest::Client, options: HttpStoreOptions) -> Self {
    Self {
      client,
      options,
    }
  }

  async fn send(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response, ResolutionError> {
    use backon::{Retryable, ExponentialBuilder};
    let req = req.build().unwrap();
    let response = (|| async {
      self.client.execute(req.try_clone().expect("Could not clone request")).await
    })
      .retry(&ExponentialBuilder::default())
      .when(|e| {
        if e.is_status() {
          e.status().map(|s| s.is_server_error()).unwrap_or(false)
        } else if e.is_timeout() {
          true
        } else if e.is_connect() {
          true
        } else {
          false
        }
      })
      .await
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?;

    match response.error_for_status_ref() {
      Ok(_) => Ok(response),
      Err(e) => {
        match e.status() {
          Some(StatusCode::NOT_FOUND) => Err(ResolutionError::NotFound),
          Some(status) if status.is_client_error() => {
            match response.json::<serde_json::Value>().await {
              Ok(j) => {
                Err(ResolutionError::Fetch(j.get("error").map(|e| e.to_string()).unwrap_or(e.to_string())))
              },
              Err(_) => Err(ResolutionError::Fetch(e.to_string())),
            }
          },
          _ => Err(ResolutionError::Fetch(e.to_string()))
        }
      },
    }
  }

  fn req(&self, path: &str) -> reqwest::RequestBuilder {
    self.client.get(self.options.url.join(&path).expect("Invalid path"))
      .header(ACCEPT, "application/vnd.ipld.car, application/json;q=0.5")
      .timeout(self.options.timeout)
  }

  fn head(&self, path: &str) -> reqwest::RequestBuilder {
    self.client.head(self.options.url.join(&path).expect("Invalid path"))
      .timeout(self.options.timeout)
  }

  fn post(&self, path: &str) -> reqwest::RequestBuilder {
    self.client.post(self.options.url.join(&path).expect("Invalid path"))
      .header(CONTENT_TYPE, "application/vnd.ipld.car")
      .timeout(self.options.timeout)
  }

  fn post_json(&self, path: &str) -> reqwest::RequestBuilder {
    self.client.post(self.options.url.join(&path).expect("Invalid path"))
      .header(CONTENT_TYPE, "application/json")
      .timeout(self.options.timeout)
  }

  async fn get_tixel(&self, path: &str) -> Result<Arc<Tixel>, ResolutionError> {
    let response = self.send(self.req(&path)).await?;
    let tixel = self.parse(response).await?.try_into()?;
    Ok(Arc::new(tixel))
  }

  async fn fetch_tixel_range(&self, strand: &Cid, upper: u64, lower: u64) -> Result<reqwest::Response, ResolutionError> {
    let path = format!("chains/{}/pulses/{}-{}", strand, upper, lower);
    self.send(self.req(&path)).await
  }

  async fn parse(&self, response: reqwest::Response) -> Result<AnyTwine, ResolutionError> {
    let tp = response.headers().get(CONTENT_TYPE).map(|h| h.to_str().unwrap_or("")).unwrap_or("");
    match tp {
      "application/vnd.ipld.car"|"application/octet-stream" => {
        let bytes = response.bytes().await.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let mut reader = CarReader::new_unchecked(bytes.as_ref()).await.map_err(|e| ResolutionError::BadData(e.to_string()))?;
        let block = reader.next_block().await
          .map_err(|e| ResolutionError::BadData(e.to_string()))?
          .ok_or(ResolutionError::BadData("No blocks found in response".to_string()))?;
        let cid = Cid::try_from(block.cid.to_bytes()).unwrap();
        let twine = AnyTwine::from_block(cid, block.data).map_err(|e| ResolutionError::Invalid(e))?;
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

  async fn parse_collection_response(&self, response: reqwest::Response) -> Result<impl Stream<Item = Result<AnyTwine, ResolutionError>>, ResolutionError> {
    let tp = response.headers().get(CONTENT_TYPE).map(|h| h.to_str().unwrap_or("")).unwrap_or("");
    use futures::stream::StreamExt;
    match tp {
      "application/vnd.ipld.car"|"application/octet-stream" => {
        let async_read = response.bytes_stream()
          .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
          .into_async_read();
        let reader = CarReader::new_unchecked(async_read).await
          .map_err(|e| ResolutionError::BadData(e.to_string()))?;
        let stream = futures::stream::unfold(reader, |mut reader| async {
          match reader.next_block().await {
            Ok(Some(block)) => {
              let cid = Cid::try_from(block.cid.to_bytes()).unwrap();
              match AnyTwine::from_block(cid, block.data) {
                Ok(twine) => Some((Ok(twine), reader)),
                Err(e) => Some((Err(ResolutionError::Invalid(e)), reader)),
              }
            },
            Ok(None) => None,
            Err(e) => Some((Err(ResolutionError::BadData(e.to_string())), reader)),
          }
        }).boxed();
        Ok(stream)
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
impl BaseResolver for HttpStore {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    let path = format!("chains/{}/pulses/{}", strand.as_cid(), index);
    let response = self.send(self.head(&path)).await?;
    Ok(response.status() == StatusCode::OK)
  }

  async fn has_twine(&self, strand: &Cid, tixel: &Cid) -> Result<bool, ResolutionError> {
    let path = format!("chains/{}/pulses/{}", strand.as_cid(), tixel.as_cid());
    let response = self.send(self.head(&path)).await?;
    Ok(response.status() == StatusCode::OK)
  }

  async fn has_strand(&self, strand: &Cid) -> Result<bool, ResolutionError> {
    let path = format!("chains/{}", strand.as_cid());
    let response = self.send(self.head(&path)).await?;
    Ok(response.status() == StatusCode::OK)
  }

  async fn strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let response = self.send(self.req("chains")).await?;
    use futures::stream::StreamExt;
    let stream = self.parse_collection_response(response).await?;
    let stream = stream.map(|t| {
      let strand = Arc::<Strand>::try_from(t?)?;
      Ok(strand)
    });
    Ok(stream.boxed())
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    let cid = strand.as_cid();
    let path = format!("chains/{}", cid);
    let response = self.send(self.req(&path)).await?;
    Ok(self.parse_expect(cid, response).await?.try_into()?)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let path = format!("chains/{}/pulses/{}", strand.as_cid(), tixel.as_cid());
    self.get_tixel(&path).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    let path = format!("chains/{}/pulses/{}", strand.as_cid(), index);
    let tixel = self.get_tixel(&path).await?;
    if tixel.index() != index {
      return Err(ResolutionError::BadData(format!("Expected index {}, found {}", index, tixel.index())));
    }
    Ok(tixel)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let path = format!("chains/{}/pulses/latest", strand.as_cid());
    let tixel = self.get_tixel(&path).await?;
    Ok(tixel)
  }

  async fn range_stream(&self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + '_>>, ResolutionError> {
    use futures::stream::StreamExt;
    let decreasing = range.is_decreasing();

    let stream = futures::stream::iter(range.batches(100))
      .map(move |range| {
        let strand_cid = range.strand.clone();
        let (upper, lower) = if decreasing {
          (range.start - 1, range.end)
        } else {
          (range.end - 1, range.start)
        };
        async move {
          let response = self.fetch_tixel_range(&strand_cid, upper, lower).await;
          if decreasing {
            Ok::<_, ResolutionError>(self.parse_collection_response(response?).await?.boxed())
          } else {
            let tixels = self.parse_collection_response(response?).await?.collect::<Vec<Result<AnyTwine, ResolutionError>>>().await;
            Ok::<_, ResolutionError>(futures::stream::iter(tixels.into_iter().rev()).boxed())
          }
        }
      })
      .buffered(self.options.concurency)
      .try_flatten()
      .then(|t| async {
        let t = t?;
        let t = Arc::<Tixel>::try_from(t)?;
        Ok(t)
      });
    Ok(stream.boxed())
  }
}

#[async_trait]
impl Store for HttpStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    let twine = twine.into();
    let strand_cid = twine.strand_cid();
    let path = match twine {
      AnyTwine::Tixel(_) => format!("chains/{}/pulses", strand_cid),
      AnyTwine::Strand(_) => format!("chains"),
    };
    let res = self.send(
        self.post_json(&path)
          .body(twine.dag_json())
      )
      .await;
    handle_save_result(res)
  }

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    use twine_core::car::to_car_stream;
    use futures::stream::StreamExt;
    let twines: Vec<AnyTwine> = twines.into_iter().map(|t| t.into()).collect();
    let (strands, tixels): (Vec<_>, Vec<_>) = twines.into_iter().partition(|t| matches!(t, AnyTwine::Strand(_)));
    if strands.len() > 0 {
      futures::stream::iter(strands).map(|strand| async move {
        self.save(strand).await
      })
      .buffered(self.options.concurency)
      .try_collect::<Vec<()>>().await?;
    }
    if tixels.len() > 0 {
      use itertools::Itertools;
      let groups_by_strand = tixels.iter()
        .map(|t| Tixel::try_from(t.to_owned()).unwrap())
        .chunk_by(|t| t.strand_cid().clone())
        .into_iter()
        .map(|(cid, it)|
          (
            cid,
            it.sorted_by(|a, b| a.index().cmp(&b.index()))
              .collect()
          )
        )
        .collect::<Vec<(_, Vec<Tixel>)>>();
      futures::stream::iter(groups_by_strand).then(|(strand_cid, group)| async move {
        let roots = vec![group.first().unwrap().cid()];
        let data = to_car_stream(futures::stream::iter(group), roots);
        // let vec = data.collect::<Vec<_>>().await;
        let path = format!("chains/{}/pulses", strand_cid);
        let res = self.send(
            self.post(&path)
              .body(reqwest::Body::wrap_stream(data.map(|b| Ok::<_, reqwest::Error>(b))))
          )
          .await;
        handle_save_result(res)
      })
      .try_collect().await?;
    }
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(&self, twines: T) -> Result<(), StoreError> {
    use futures::stream::StreamExt;
    self.save_many(twines.collect::<Vec<_>>().await).await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
    let res = self.send(
        self.client.delete(
          self.options.url.join(&format!("cid/{}", cid.as_cid())).expect("Invalid path")
        )
      ).await;
    handle_save_result(res)
  }
}
