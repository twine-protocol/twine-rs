
pub(crate) mod bytes_base64 {
    use serde::{Serialize, Deserialize};
    use serde::{Deserializer, Serializer};
    use base64::{engine::general_purpose::URL_SAFE, Engine};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = URL_SAFE.encode(v);
        String::serialize(&base64, s)
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64 = String::deserialize(d)?;
        URL_SAFE.decode(base64.as_bytes())
            .map_err(|e| serde::de::Error::custom(e))
    }
}

pub(crate) mod dag_json {
    
}