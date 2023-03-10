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

use reqwest::header::{HeaderMap, HeaderName};
use reqwest::Url;
use serde::de::Error as DeserError;
use serde::{Deserialize, Deserializer};

use super::super::{Error, ErrorKind};

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

/// Get a header as a string.
#[inline]
pub fn get_header<'m>(headers: &'m HeaderMap, key: &HeaderName) -> Result<Option<&'m str>, Error> {
    Ok(if let Some(hdr) = headers.get(key) {
        Some(hdr.to_str().map_err(|e| {
            Error::new(
                ErrorKind::InvalidResponse,
                format!("{} header is invalid string: {}", key.as_str(), e),
            )
        })?)
    } else {
        None
    })
}

/// Get a header as a string, failing if it's not present.
#[inline]
pub fn get_required_header<'m>(headers: &'m HeaderMap, key: &HeaderName) -> Result<&'m str, Error> {
    get_header(headers, key)?.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidResponse,
            format!("Missing {} header", key.as_str()),
        )
    })
}
