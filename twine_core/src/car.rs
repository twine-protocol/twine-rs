use std::error::Error;
use futures::Stream;
use futures::stream::StreamExt;
use libipld::Cid;
use ipld_core::codec::Codec;
use serde_ipld_dagcbor::codec::DagCborCodec;
use serde::{Deserialize, Serialize};
use crate::prelude::TwineBlock;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarHeader {
  pub version: u8,
  pub roots: Vec<Cid>,
}

pub fn write_car_stream<I: TwineBlock, S: Stream<Item=I>>(stream: S, roots: Vec<Cid>) -> impl Stream<Item=Vec<u8>> {
  let header = CarHeader {
    version: 1,
    roots,
  };
  let header_bytes = DagCborCodec::encode_to_vec(&header).unwrap();
  let mut buf = [0u8; U64_LEN];
  let (buf_ref, input_len) = encode_varint_u64(header_bytes.len() as u64, &mut buf);
  let (enc, _) = buf_ref.split_at(input_len);
  let header = vec!(enc.to_vec(), header_bytes).concat();
  let blocks = stream.map(|twine| {
    let cid = twine.cid();
    let bytes = twine.bytes();
    let mut buf = [0u8; U64_LEN];
    let (buf_ref, len) = encode_varint_u64((bytes.len() + cid.encoded_len()) as u64, &mut buf);
    let (enc, _) = buf_ref.split_at(len);
    vec![
      enc,
      &cid.to_bytes(),
      &bytes,
    ].concat()
  });
  futures::stream::iter(vec![header])
    .chain(blocks)
}

#[cfg(test)]
mod test {
  use super::*;
  use async_std::io::Cursor;
use rs_car::CarReader;
  use crate::prelude::*;
  use crate::test::STRANDJSON;

  #[tokio::test]
  async fn test_write_car_stream() -> Result<(), Box<dyn Error>> {
    let twine = Strand::from_dag_json(STRANDJSON).unwrap();
    let stream = futures::stream::iter(vec![twine.clone()]);
    let roots = vec![twine.cid()];
    let car_stream = write_car_stream(stream, roots.clone());
    let car_bytes = car_stream.collect::<Vec<_>>().await.concat();
    let mut cursor = Cursor::new(car_bytes);
    let mut reader = CarReader::new(&mut cursor, false).await?;
    let header = &reader.header;
    assert_eq!(header.version as u8, 1);
    assert_eq!(header.roots, roots);
    let (cid, bytes) = reader.next().await.unwrap().unwrap();
    assert_eq!(cid, twine.cid());
    assert_eq!(*bytes, *twine.bytes());
    Ok(())
  }
}
