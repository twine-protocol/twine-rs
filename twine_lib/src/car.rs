use crate::twine::TwineBlock;
use crate::{errors::VerificationError, twine::AnyTwine, Cid};
use futures::stream::StreamExt;
use futures::Stream;
use ipld_core::codec::Codec;
use rs_car_sync::CarReader;
use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor::codec::DagCborCodec;
use std::io::Read;

/// Error type for car decoding
#[derive(Debug, thiserror::Error)]
pub enum CarDecodeError {
  /// Invalid Twine data
  #[error("{0}")]
  VerificationError(#[from] VerificationError),
  /// Error decoding CAR
  #[error("Error decoding CAR: {0}")]
  DecodeError(#[from] rs_car_sync::CarDecodeError),
}

// Max size of u64 varint
const U64_LEN: usize = 10;

// Implementation copied from https://github.com/paritytech/unsigned-varint/blob/a3a5b8f2bee1f44270629e96541adf805a53d32c/src/encode.rs#L22
fn encode_varint_u64(input: u64, buf: &mut [u8; U64_LEN]) -> (&[u8], usize) {
  let mut n = input;
  let mut i = 0;
  for b in buf.iter_mut() {
    *b = n as u8 | 0b1000_0000;
    n >>= 7;
    if n == 0 {
      *b &= 0b0111_1111;
      break;
    }
    i += 1
  }
  debug_assert_eq!(n, 0);
  (&buf[0..=i], i + 1)
}

/// A CAR header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarHeader {
  /// The version of the CAR format
  pub version: u8,
  /// The root CIDs
  pub roots: Vec<Cid>,
}

/// Convert a stream of TwineBlocks to a CAR stream
///
/// # Example
///
/// ```rust,no_run
/// use twine_lib::car::to_car_stream;
/// # use twine_lib::twine::{Strand, Tixel};
/// # use twine_lib::twine::TwineBlock;
/// # use twine_lib::{errors::VerificationError, Cid};
/// # use std::{fmt::Display, sync::Arc};
/// # use futures::stream::StreamExt;
/// # #[derive(Clone)]
/// # struct Dummy(Cid);
/// # impl TwineBlock for Dummy {
/// #   fn cid(&self) -> &Cid { &self.0 }
/// #   fn bytes(&self) -> Arc<[u8]> { Arc::new([]) }
/// #   fn from_tagged_dag_json<S: Display>(_json: S) -> Result<Self, VerificationError> { Ok(Dummy(Cid::default())) }
/// #   fn from_bytes_unchecked(_hasher: multihash_codetable::Code, _bytes: Vec<u8>) -> Result<Self, VerificationError> { Ok(Dummy(Cid::default())) }
/// #   fn from_block<T: AsRef<[u8]>>(_cid: Cid, _bytes: T) -> Result<Self, VerificationError> { Ok(Dummy(_cid)) }
/// #   fn tagged_dag_json(&self) -> String { "".to_string() }
/// #   fn content_bytes(&self) -> Arc<[u8]> { Arc::new([]) }
/// # }
/// # let some_strand = Dummy(Cid::default());
///
/// let stream = futures::stream::iter(vec![some_strand.clone()]);
/// let roots = vec![some_strand.cid().clone()];
/// let car_stream = to_car_stream(stream, roots);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let bytes: Vec<u8> = car_stream.collect::<Vec<_>>().await.concat();
/// # });
/// ```
pub fn to_car_stream<I: TwineBlock, S: Stream<Item = I>>(
  stream: S,
  roots: Vec<Cid>,
) -> impl Stream<Item = Vec<u8>> {
  let header = CarHeader { version: 1, roots };
  let header_bytes = DagCborCodec::encode_to_vec(&header).unwrap();
  let mut buf = [0u8; U64_LEN];
  let (buf_ref, input_len) = encode_varint_u64(header_bytes.len() as u64, &mut buf);
  let (enc, _) = buf_ref.split_at(input_len);
  let header = vec![enc.to_vec(), header_bytes].concat();
  let blocks = stream.map(|twine| {
    let cid = twine.cid();
    let bytes = twine.bytes();
    let mut buf = [0u8; U64_LEN];
    let (buf_ref, len) = encode_varint_u64((bytes.len() + cid.encoded_len()) as u64, &mut buf);
    let (enc, _) = buf_ref.split_at(len);
    vec![enc, &cid.to_bytes(), &bytes].concat()
  });
  futures::stream::iter(vec![header]).chain(blocks)
}

/// Convert a CAR stream of bytes to a stream of TwineBlocks
///
pub fn from_car_bytes<R: Read>(mut reader: &mut R) -> Result<Vec<AnyTwine>, CarDecodeError> {
  // block validation happens in twine creation
  let car_reader = CarReader::new(&mut reader, false)?;
  car_reader
    .map(|result| -> Result<AnyTwine, CarDecodeError> {
      let (cid, bytes) = result?;
      let cid = Cid::read_bytes(&*cid.to_bytes()).expect("cid should be valid format");
      let twine = AnyTwine::from_block(cid, bytes)?;
      Ok(twine)
    })
    .collect()
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::test::STRANDJSON;
  use crate::twine::*;
  use futures::io::Cursor;
  use rs_car::CarReader;
  use std::error::Error;

  #[tokio::test]
  async fn test_to_car_stream() -> Result<(), Box<dyn Error>> {
    let twine = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let stream = futures::stream::iter(vec![twine.clone()]);
    let roots = vec![twine.cid()];
    let car_stream = to_car_stream(stream, roots.clone());
    let car_bytes = car_stream.collect::<Vec<_>>().await.concat();
    let mut cursor = Cursor::new(car_bytes);
    let mut reader = CarReader::new(&mut cursor, false).await?;
    let header = &reader.header;
    assert_eq!(header.version as u8, 1);
    assert_eq!(header.roots[0].to_bytes(), roots[0].to_bytes());
    let (cid, bytes) = reader.next().await.unwrap().unwrap();
    assert_eq!(cid.to_bytes(), twine.cid().to_bytes());
    assert_eq!(*bytes, *twine.bytes());
    Ok(())
  }

  #[tokio::test]
  async fn test_from_car_bytes() -> Result<(), Box<dyn Error>> {
    let twine = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let roots = vec![twine.cid()];
    let stream = futures::stream::iter(vec![twine.clone()]);
    let car_stream = to_car_stream(stream, roots.clone());
    let car_bytes = car_stream.collect::<Vec<_>>().await.concat();
    let twines = from_car_bytes(&mut &*car_bytes).unwrap();
    assert_eq!(twines.len(), 1);
    assert_eq!(twines[0].cid(), twine.cid());
    assert_eq!(twines[0].bytes(), twine.bytes());
    Ok(())
  }
}
