use libipld::{cid::{Cid, Version}, multibase::Base};
use serde::{Serializer, ser::SerializeStruct, Deserializer, de::Visitor};

pub fn serialize<S>(cid: Cid, ser: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let encoded = match cid.version() {
        Version::V0 => cid.to_string_of_base(Base::Base32Lower), // V0 is encoded as B32
        Version::V1 => cid.to_string_of_base(Base::Base58Btc) // TODO: BTC base58?
    };

    let mut struct_ser = ser.serialize_struct("Cid", 1)?;
    struct_ser.serialize_field("/", &encoded);
    struct_ser.end()
}


pub fn deserialize<'de, D>(de: D) -> Result<Cid, D::Error> where D: Deserializer<'de> {
    de.deserialize_struct("Cid", &["/"], Visitor);
    unimplemented!()    
}