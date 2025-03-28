//! Provides an HTTP store for the version 2 HTTP api
use async_trait::async_trait;
use futures::stream::{StreamExt, TryStreamExt};
use futures::Stream;
use reqwest::{
  header::{ACCEPT, CONTENT_TYPE},
  Method, StatusCode, Url,
};
use twine_lib::car::from_car_bytes;
use twine_lib::resolver::unchecked_base::TwineStream;
use twine_lib::resolver::{MaybeSend, Resolver, TwineResolution};
use twine_lib::twine::Twine;
use twine_lib::{
  as_cid::AsCid,
  errors::{ResolutionError, StoreError},
  resolver::{unchecked_base::BaseResolver, AbsoluteRange, SingleQuery},
  store::Store,
  twine::{AnyTwine, Strand, Tixel},
  Cid,
};

fn handle_save_result(res: Result<reqwest::Response, ResolutionError>) -> Result<(), StoreError> {
  match res {
    Ok(_) => Ok::<(), StoreError>(()),
    Err(e) => match e {
      ResolutionError::Fetch(e) => Err(StoreError::Saving(e)),
      ResolutionError::NotFound => Err(StoreError::Saving("Not found".to_string())),
      ResolutionError::Invalid(e) => Err(StoreError::Invalid(e)),
      ResolutionError::BadData(e) => Err(StoreError::Saving(e)),
      ResolutionError::QueryMismatch(q) => {
        Err(StoreError::Saving(format!("SingleQuery mismatch: {}", q)))
      }
    },
  }
}

pub(crate) async fn parse_response(
  response: reqwest::Response,
) -> Result<impl Stream<Item = Result<AnyTwine, ResolutionError>>, ResolutionError> {
  let reader = response
    .bytes()
    .await
    .map_err(|e| ResolutionError::Fetch(e.to_string()))?;
  use twine_lib::car::CarDecodeError;
  let twines = from_car_bytes(&mut reader.as_ref()).map_err(|e| match e {
    CarDecodeError::DecodeError(e) => ResolutionError::BadData(e.to_string()),
    CarDecodeError::VerificationError(e) => ResolutionError::Invalid(e),
  })?;
  let stream = futures::stream::iter(twines.into_iter().map(Ok));
  Ok(stream)
}

pub(crate) async fn type_from_response<E, T: TryFrom<AnyTwine, Error = E>>(
  response: reqwest::Response,
) -> Result<T, ResolutionError>
where
  ResolutionError: From<E>,
{
  let mut stream = parse_response(response).await?;
  let first = stream
    .next()
    .await
    .ok_or(ResolutionError::BadData("No data in response".into()))?;
  let item = T::try_from(first?)?;
  Ok(item)
}

pub(crate) async fn twine_from_response(
  response: reqwest::Response,
) -> Result<Twine, ResolutionError> {
  let mut stream = parse_response(response).await?;
  let first = stream
    .next()
    .await
    .ok_or(ResolutionError::BadData("No data in response".into()))?;
  let second = stream.next().await.ok_or(ResolutionError::BadData(
    "Expected more data in response".into(),
  ))?;

  let (strand, tixel) = match (first?, second?) {
    (AnyTwine::Strand(s), AnyTwine::Tixel(t)) => (s, t),
    (AnyTwine::Tixel(t), AnyTwine::Strand(s)) => (s, t),
    _ => return Err(ResolutionError::BadData("Expected Strand and Tixel".into())),
  };

  Ok(Twine::try_new(strand, tixel)?)
}

/// A type implementing the [`Store`] trait for the version 2 HTTP API
#[derive(Debug, Clone)]
pub struct HttpStore {
  client: reqwest::Client,
  url: Url,
  concurency: usize,
  batch_size: u64,
}

impl Default for HttpStore {
  fn default() -> Self {
    Self::new(reqwest::Client::new())
  }
}

impl HttpStore {
  /// Create a new instance of the HTTP store
  ///
  /// You can customize the client with the `reqwest::Client::builder()` method.
  ///
  /// # Example
  ///
  /// ```rust
  /// use twine_http_store::v2::HttpStore;
  /// use twine_http_store::reqwest;
  /// let client = reqwest::Client::builder()
  ///   .timeout(std::time::Duration::from_secs(10))
  ///   .build()
  ///   .unwrap();
  /// let store = HttpStore::new(client)
  ///   .with_url("http://localhost:8080");
  /// ```
  pub fn new(client: reqwest::Client) -> Self {
    Self {
      client,
      url: Url::parse("http://localhost:8080").unwrap(),
      concurency: 10,
      batch_size: 1000,
    }
  }

  /// Mutate the URL for the store
  pub fn url(&mut self, url: &str) -> &mut Self {
    self.url = format!("{}/", url).parse().expect("Invalid URL");
    self
  }

  /// Set a new URL for the store and return the updated instance
  pub fn with_url(mut self, url: &str) -> Self {
    self.url = format!("{}/", url).parse().expect("Invalid URL");
    self
  }

  /// Mutate the concurency for the store
  ///
  /// The concurency is the number of concurrent HTTP requests that
  /// can be made to the server
  pub fn concurency(&mut self, concurency: usize) -> &mut Self {
    self.concurency = concurency;
    self
  }

  /// Set a new concurency for the store and return the updated instance
  ///
  /// The concurency is the number of concurrent HTTP requests that
  /// can be made to the server
  pub fn with_concurency(mut self, concurency: usize) -> Self {
    self.concurency = concurency;
    self
  }

  /// Mutate the batch size for the store
  ///
  /// The batch size is the number of items to include in a single
  /// request to the server
  pub fn batch_size(&mut self, batch_size: u64) -> &mut Self {
    self.batch_size = batch_size;
    self
  }

  /// Set a new batch size for the store and return the updated instance
  ///
  /// The batch size is the number of items to include in a single
  /// request to the server
  pub fn with_batch_size(mut self, batch_size: u64) -> Self {
    self.batch_size = batch_size;
    self
  }

  // pub async fn register(&self, reg: Registration) -> Result<(), StoreError> {
  //   let req = self.post("register").json(&reg);
  //   let res = self.send(req).await;
  //   handle_save_result(res)
  // }

  fn req(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
    let mut url = self.url.clone();
    url.set_path(path);
    self
      .client
      .request(method, url)
      .header(ACCEPT, "application/vnd.ipld.car")
  }

  fn head(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::HEAD, path)
  }

  fn get(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::GET, path)
  }

  #[allow(dead_code)]
  fn post(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::POST, path)
  }

  #[allow(dead_code)]
  fn put(&self, path: &str) -> reqwest::RequestBuilder {
    self.req(Method::PUT, path)
  }

  fn put_car(&self, path: &str) -> reqwest::RequestBuilder {
    self
      .req(Method::PUT, path)
      .header(CONTENT_TYPE, "application/vnd.ipld.car")
  }

  async fn send(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response, ResolutionError> {
    use backon::{ExponentialBuilder, Retryable};
    let req = req.build().unwrap();
    let response = (|| async {
      self
        .client
        .execute(req.try_clone().expect("Could not clone request"))
        .await
    })
    .retry(ExponentialBuilder::default())
    .when(|e| {
      if e.is_status() {
        e.status().map(|s| s.is_server_error()).unwrap_or(false)
      } else if e.is_timeout() {
        true
      } else {
        false
      }
    })
    .await
    .map_err(|e| ResolutionError::Fetch(e.to_string()))?;

    match response.error_for_status_ref() {
      Ok(_) => Ok(response),
      Err(e) => match e.status() {
        Some(StatusCode::NOT_FOUND) => Err(ResolutionError::NotFound),
        Some(status) if status.is_client_error() => {
          let body = response.text().await.unwrap_or(e.to_string());
          match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(j) => Err(ResolutionError::Fetch(
              j.get("error")
                .map(|e| e.to_string())
                .unwrap_or(e.to_string()),
            )),
            Err(_) => {
              if body.len() > 0 {
                Err(ResolutionError::Fetch(body))
              } else {
                Err(ResolutionError::Fetch(e.to_string()))
              }
            }
          }
        }
        _ => Err(ResolutionError::Fetch(e.to_string())),
      },
    }
  }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BaseResolver for HttpStore {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    let q: SingleQuery = (strand, index).into();
    let path = format!("{}", q);
    match self.send(self.head(&path)).await {
      Ok(response) => Ok(response.status() == StatusCode::OK),
      Err(ResolutionError::NotFound) => Ok(false),
      Err(e) => Err(e),
    }
  }

  async fn has_twine(&self, strand: &Cid, tixel: &Cid) -> Result<bool, ResolutionError> {
    let q: SingleQuery = (strand, tixel).into();
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

  async fn fetch_strands(&self) -> Result<TwineStream<'_, Strand>, ResolutionError> {
    let response = self.send(self.get("")).await?;
    let stream = parse_response(response).await?;
    let stream = stream.map(|t| {
      let strand = Strand::try_from(t?)?;
      Ok(strand)
    });
    #[cfg(target_arch = "wasm32")]
    {
      Ok(stream.boxed_local())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      Ok(stream.boxed())
    }
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    let cid = strand.as_cid();
    let path = format!("{}", cid);
    let response = self.send(self.get(&path)).await?;
    let strand = type_from_response(response).await?;
    Ok(strand)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    let q: SingleQuery = (strand, tixel).into();
    let path = format!("{}", q);
    let response = self.send(self.get(&path)).await?;
    let tixel = type_from_response(response).await?;
    Ok(tixel)
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    let q: SingleQuery = (strand, index).into();
    let path = format!("{}", q);
    let response = self.send(self.get(&path)).await?;
    let tixel = type_from_response(response).await?;
    Ok(tixel)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    let q = SingleQuery::Latest(*strand);
    let path = format!("{}", q);
    let response = self.send(self.get(&path)).await?;
    let tixel = type_from_response(response).await?;
    Ok(tixel)
  }

  async fn range_stream(
    &self,
    range: AbsoluteRange,
  ) -> Result<TwineStream<'_, Tixel>, ResolutionError> {
    use futures::stream::StreamExt;
    let stream = futures::stream::iter(range.batches(self.batch_size))
      .map(move |range| {
        let path = format!("{}", range);
        async move {
          let res = self.send(self.get(&path)).await?;
          parse_response(res).await
        }
      })
      .buffered(self.concurency)
      .try_flatten()
      .then(|t| async {
        let t = t?;
        let t = Tixel::try_from(t)?;
        Ok(t)
      });
    #[cfg(target_arch = "wasm32")]
    {
      Ok(stream.boxed_local())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      Ok(stream.boxed())
    }
  }
}

// optimized implementations
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Resolver for HttpStore {
  async fn resolve_latest<C: AsCid + MaybeSend>(
    &self,
    strand: C,
  ) -> Result<TwineResolution, ResolutionError> {
    let q = SingleQuery::from(*strand.as_cid());
    let path = format!("{}", q);
    let response = self.send(self.get(&path).query(&[("full", "")])).await?;
    let twine = twine_from_response(response).await?;
    TwineResolution::try_new(q, twine)
  }

  async fn resolve_index<C: AsCid + MaybeSend>(
    &self,
    strand: C,
    index: u64,
  ) -> Result<TwineResolution, ResolutionError> {
    let q = SingleQuery::from((strand.as_cid(), index));
    let path = format!("{}", q);
    let response = self.send(self.get(&path).query(&[("full", "")])).await?;
    let twine = twine_from_response(response).await?;
    TwineResolution::try_new(q, twine)
  }

  async fn resolve_stitch<C: AsCid + MaybeSend>(
    &self,
    strand: C,
    tixel: C,
  ) -> Result<TwineResolution, ResolutionError> {
    let q = SingleQuery::from((strand.as_cid(), tixel.as_cid()));
    let path = format!("{}", q);
    let response = self.send(self.get(&path).query(&[("full", "")])).await?;
    let twine = twine_from_response(response).await?;
    TwineResolution::try_new(q, twine)
  }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Store for HttpStore {
  async fn save<T: Into<AnyTwine> + MaybeSend>(&self, twine: T) -> Result<(), StoreError> {
    let twine = twine.into();
    self.save_many(vec![twine]).await
  }

  async fn save_many<
    I: Into<AnyTwine> + MaybeSend,
    S: Iterator<Item = I> + MaybeSend,
    T: IntoIterator<Item = I, IntoIter = S> + MaybeSend,
  >(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    use futures::stream::StreamExt;
    use twine_lib::car::to_car_stream;
    let twines: Vec<AnyTwine> = twines.into_iter().map(|t| t.into()).collect();
    let (strands, tixels): (Vec<_>, Vec<_>) = twines
      .into_iter()
      .partition(|t| matches!(t, AnyTwine::Strand(_)));
    if strands.len() > 0 {
      let jobs = strands
        .into_iter()
        .map(|strand| async {
          let strand_cid = strand.cid();
          let path = "".to_string();
          let data = to_car_stream(futures::stream::iter(vec![strand]), vec![strand_cid]);
          let items = data.collect::<Vec<_>>().await.concat();
          let res = self.send(self.put_car(&path).body(items)).await;
          handle_save_result(res)
        })
        .collect::<Vec<_>>();

      futures::stream::iter(jobs)
        .buffered(self.concurency)
        .try_for_each(|_| async { Ok(()) })
        .await?;
    }

    if tixels.len() > 0 {
      use itertools::Itertools;
      let groups_by_strand = tixels
        .iter()
        .map(|t| Tixel::try_from(t.to_owned()).unwrap())
        .chunk_by(|t| t.strand_cid().clone())
        .into_iter()
        .map(|(cid, it)| {
          (
            cid,
            it.sorted_by(|a, b| a.index().cmp(&b.index()))
              .chunks(self.batch_size as usize)
              .into_iter()
              .map(|g| g.collect())
              .collect::<Vec<Vec<_>>>(),
          )
        })
        .collect::<Vec<_>>();

      let jobs = groups_by_strand
        .into_iter()
        .map(|(strand_cid, group)| {
          let strand_cid = strand_cid.clone();
          group.into_iter().map(move |group| async move {
            let path = format!("{}", strand_cid);
            let roots = vec![group.first().unwrap().cid()];
            let data = to_car_stream(futures::stream::iter(group), roots);
            let items = data.collect::<Vec<_>>().await.concat();
            let res = self.send(self.put_car(&path).body(items)).await;
            handle_save_result(res)
          })
        })
        .flatten();

      futures::stream::iter(jobs)
        .buffered(self.concurency)
        .try_for_each(|_| async { Ok(()) })
        .await?;
    }
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + MaybeSend, T: Stream<Item = I> + MaybeSend + Unpin>(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    use futures::stream::StreamExt;
    twines
      .chunks(self.batch_size as usize)
      .then(|chunk| self.save_many(chunk))
      .try_for_each(|_| async { Ok(()) })
      .await?;
    Ok(())
  }

  async fn delete<C: AsCid + MaybeSend>(&self, _cid: C) -> Result<(), StoreError> {
    unimplemented!("delete")
  }
}
