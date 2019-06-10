// Copyright 2018 Dmitry Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Common protocol bits.

#![allow(dead_code)] // various things are unused with --no-default-features
#![allow(missing_docs)]

use std::collections::HashMap;

use eui48::MacAddress;
use reqwest::Url;
use serde::de::Error as DeserError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

/// Deserialize a URL.
pub fn deser_optional_url<'de, D>(des: D) -> ::std::result::Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(des)?;
    match value {
        Some(s) => Url::parse(&s).map_err(DeserError::custom).map(Some),
        None => Ok(None),
    }
}

/// Deserialize a key-value mapping.
pub fn deser_key_value<'de, D>(des: D) -> ::std::result::Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Vec<KeyValue> = Deserialize::deserialize(des)?;
    Ok(value.into_iter().map(|kv| (kv.key, kv.value)).collect())
}

/// Serialize a MAC address in its HEX format.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn ser_mac<S>(value: &MacAddress, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.to_hex_string().serialize(serializer)
}

/// Serialize a MAC address in its HEX format.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn ser_opt_mac<S>(
    value: &Option<MacAddress>,
    serializer: S,
) -> ::std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.map(|m| m.to_hex_string()).serialize(serializer)
}
