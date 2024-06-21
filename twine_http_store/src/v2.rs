use std::{pin::Pin, sync::Arc, time::Duration};
use async_trait::async_trait;
use futures::stream::{StreamExt, TryStreamExt};
use futures::Stream;
use fvm_ipld_car::CarReader;
use reqwest::{header::ACCEPT, Method, StatusCode, Url};
use serde::{Deserialize, Serialize};
use twine_core::resolver::Resolver;
use twine_core::twine::{Twine, TwineBlock};
use twine_core::{as_cid::AsCid, errors::{ResolutionError, StoreError}, resolver::{AbsoluteRange, BaseResolver, Query}, store::Store, twine::{AnyTwine, Strand, Tixel}, Cid};
use twine_core::serde::dag_json;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
  pub strand: Strand,
  #[serde(with = "dag_json")]
  pub email: String,
}

#[derive(Debug, Clone)]
pub struct HttpStore {
  client: reqwest::Client,
  url: Url,
  timeout: Duration,
  concurency: usize,
  batch_size: u64,
}

impl Default for HttpStore {
  fn default() -> Self {
    Self::new(
      reqwest::Client::new(),
    )
  }
}

impl HttpStore {
  pub fn new(client: reqwest::Client) -> Self {
    Self {
      client,
      url: Url::parse("http://localhost:8080").unwrap(),
      timeout: Duration::from_secs(30),
      concurency: 10,
      batch_size: 1000,
    }
  }

  pub fn url(&mut self, url: &str) -> &mut Self {
    self.url = format!("{}/", url).parse().expect("Invalid URL");
    self
  }

  pub fn with_url(mut self, url: &str) -> Self {
    self.url = format!("{}/", url).parse().expect("Invalid URL");
    self
  }

  pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
    self.timeout = timeout;
    self
  }

  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  pub fn concurency(&mut self, concurency: usize) -> &mut Self {
    self.concurency = concurency;
    self
  }

  pub fn with_concurency(mut self, concurency: usize) -> Self {
    self.concurency = concurency;
    self
  }

  pub fn batch_size(&mut self, batch_size: u64) -> &mut Self {
    self.batch_size = batch_size;
    self
  }

  pub fn with_batch_size(mut self, batch_size: u64) -> Self {
    self.batch_size = batch_size;
    self
  }

  pub async fn register(&self, reg: Registration) -> Result<(), StoreError> {
    let req = self.post("register").json(&reg);
    let res = self.send(req).await;
    handle_save_result(res)
  }

  fn req(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
    let mut url = self.url.clone();
    url.set_path(path);
    self.client.request(method, url)
      .header(ACCEPT, "application/vnd.ipld.car")
      .timeout(self.timeout)
  }

  fn head(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::HEAD, path)
  }

  fn get(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::GET, path)
  }

  fn post(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::POST, path)
  }

  fn put(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::PUT, path)
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

  async fn parse_response(&self, response: reqwest::Response) -> Result<impl Stream<Item = Result<AnyTwine, ResolutionError>>, ResolutionError> {
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
  }

  async fn type_from_response<E, T: TryFrom<AnyTwine, Error = E>>(&self, response: reqwest::Response) -> Result<T, ResolutionError> where ResolutionError: From<E> {
    let mut stream = self.parse_response(response).await?;
    let first = stream.next().await.ok_or(ResolutionError::BadData("No data in response".into()))?;
    let item = T::try_from(first?)?;
    Ok(item)
  }

  async fn twine_from_response(&self, response: reqwest::Response) -> Result<Twine, ResolutionError> {
    let mut stream = self.parse_response(response).await?;
    let first = stream.next().await.ok_or(ResolutionError::BadData("No data in response".into()))?;
    let second = stream.next().await.ok_or(ResolutionError::BadData("Expected more data in response".into()))?;

    let (strand, tixel) = match (first?, second?) {
      (AnyTwine::Strand(s), AnyTwine::Tixel(t)) => (s, t),
      (AnyTwine::Tixel(t), AnyTwine::Strand(s)) => (s, t),
      _ => return Err(ResolutionError::BadData("Expected Strand and Tixel".into())),
    };

    Ok(Twine::try_new_from_shared(strand, tixel)?)
  }
}

#[async_trait]
impl BaseResolver for HttpStore {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    let q : Query = (strand, index).into();
    let path = format!("{}", q);
    match self.send(self.head(&path)).await {
      Ok(response) => Ok(response.status() == StatusCode::OK),
      Err(ResolutionError::NotFound) => Ok(false),
      Err(e) => Err(e),
    }
  }

  async fn has_twine(&self, strand: &Cid, tixel: &Cid) -> Result<bool, ResolutionError> {
    let q : Query = (strand, tixel).into();
    let path = format!("{}", q);
    match self.send(self.head(&path)).await {
      Ok(response) => Ok(response.status() == StatusCode::OK),
      Err(ResolutionError::NotFound) => Ok(false),
      Err(e) => Err(e),
    }
  }

  async fn has_strand(&self, strand: &Cid) -> Result<bool, ResolutionError> {
    let path = format!("{}", strand.as_cid());
    match self.send(self.head(&path)).await {
      Ok(response) => Ok(response.status() == StatusCode::OK),
      Err(ResolutionError::NotFound) => Ok(false),
      Err(e) => Err(e),
    }
  }

  async fn strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let response = self.send(self.get("")).await?;
    let stream = self.parse_response(response).await?;
    let stream = stream.map(|t| {
      let strand = Arc::<Strand>::try_from(t?)?;
      Ok(strand)
    });
    Ok(stream.boxed())
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    let cid = strand.as_cid();
    let path = format!("{}", cid);
    let response = self.send(self.get(&path)).await?;
    let strand = self.type_from_response(response).await?;
    Ok(strand)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let q : Query = (strand, tixel).into();
    let path = format!("{}", q);
    let response = self.send(self.get(&path)).await?;
    let tixel = self.type_from_response(response).await?;
    Ok(tixel)
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    let q : Query = (strand, index).into();
    let path = format!("{}", q);
    let response = self.send(self.get(&path)).await?;
    let tixel = self.type_from_response(response).await?;
    Ok(tixel)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let q = Query::Latest(*strand);
    let path = format!("{}", q);
    let response = self.send(self.get(&path)).await?;
    let tixel = self.type_from_response(response).await?;
    Ok(tixel)
  }

  async fn range_stream(&self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + '_>>, ResolutionError> {
    use futures::stream::StreamExt;
    let stream = futures::stream::iter(range.batches(self.batch_size))
      .map(move |range| {
        let path = format!("{}", range);
        async move {
          let res = self.send(self.get(&path)).await?;
          self.parse_response(res).await
        }
      })
      .buffered(self.concurency)
      .try_flatten()
      .then(|t| async {
        let t = t?;
        let t = Arc::<Tixel>::try_from(t)?;
        Ok(t)
      });
    Ok(stream.boxed())
  }
}

// optimized implementations
#[async_trait]
impl Resolver for HttpStore {
  async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
    let q = Query::from(*strand.as_cid());
    let path = format!("{}?full", q);
    let response = self.send(self.get(&path)).await?;
    let twine = self.twine_from_response(response).await?;
    Ok(twine)
  }

  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
    let q = Query::from((strand.as_cid(), index));
    let path = format!("{}?full", q);
    let response = self.send(self.get(&path)).await?;
    let twine = self.twine_from_response(response).await?;
    Ok(twine)
  }

  async fn resolve_stitch<C: AsCid + Send>(&self, strand: C, tixel: C) -> Result<Twine, ResolutionError> {
    let q = Query::from((strand.as_cid(), tixel.as_cid()));
    let path = format!("{}?full", q);
    let response = self.send(self.get(&path)).await?;
    let twine = self.twine_from_response(response).await?;
    Ok(twine)
  }
}

#[async_trait]
impl Store for HttpStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    let twine = twine.into();
    self.save_many(vec![twine]).await
  }

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    use twine_core::car::to_car_stream;
    use futures::stream::StreamExt;
    let twines: Vec<AnyTwine> = twines.into_iter().map(|t| t.into()).collect();
    let (strands, tixels): (Vec<_>, Vec<_>) = twines.into_iter().partition(|t| matches!(t, AnyTwine::Strand(_)));
    if strands.len() > 0 {
      return Err(StoreError::Saving("Strands must be saved with register()".to_string()));
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
        let path = format!("{}", strand_cid);
        let res = self.send(
            self.put(&path)
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

  async fn delete<C: AsCid + Send>(&self, _cid: C) -> Result<(), StoreError> {
    unimplemented!("delete")
  }
}
