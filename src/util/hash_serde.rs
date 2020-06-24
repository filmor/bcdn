use blake3::Hash;

use serde::de::{Error, Visitor};
use serde::{Deserializer, Serializer};

use std::convert::TryInto;
use std::fmt;

pub fn serialize<S>(data: &Hash, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = data.to_hex();
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Hash, D::Error>
where
    D: Deserializer<'de>,
{
    struct HashStrVisitor;

    impl<'de> Visitor<'de> for HashStrVisitor {
        type Value = Hash;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a hex encoded string")
        }

        fn visit_str<E>(self, data: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            let hash_bytes =
                hex::decode(data).map_err(|_e| Error::custom("Failed to decode hex"))?;
            let hash_array: [u8; blake3::OUT_LEN] = hash_bytes[..]
                .try_into()
                .map_err(|_e| Error::custom("Invalid length"))?;
            Ok(hash_array.into())
        }
    }

    deserializer.deserialize_str(HashStrVisitor)
}
