// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Various utilities.

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::str::FromStr;

use hyper::Client;
#[cfg(feature = "tls")]
use hyper::net::HttpsConnector;
#[cfg(feature = "tls")]
use hyper_rustls::TlsClient;
use serde::{Deserialize, Deserializer};
use serde::de::Error as DeserError;

use super::ApiResult;


/// Create an HTTP(s) client.
#[cfg(feature = "tls")]
pub fn http_client() -> Client {
    let connector = HttpsConnector::new(TlsClient::new());
    Client::with_connector(connector)
}

/// Create an HTTP-only client.
#[cfg(not(feature = "tls"))]
pub fn http_client() -> Client {
    Client::new()
}

/// Cached clone-able value.
#[derive(Debug, Clone)]
pub struct ValueCache<T: Clone>(RefCell<Option<T>>);

/// Cached map of values.
#[derive(Debug, Clone)]
pub struct MapCache<K: Hash + Eq, V: Clone>(RefCell<HashMap<K, V>>);


impl<T: Clone> ValueCache<T> {
    /// Create a cache.
    pub fn new(value: Option<T>) -> ValueCache<T> {
        ValueCache(RefCell::new(value))
    }

    /// Ensure the value is cached.
    pub fn ensure_value<F>(&self, default: F) -> ApiResult<()>
            where F: FnOnce() -> ApiResult<T> {
        if self.0.borrow().is_some() {
            return Ok(());
        };

        *self.0.borrow_mut() = Some(default()?);
        Ok(())
    }

    /// Get the cached value.
    #[inline]
    pub fn get(&self) -> Option<T> {
        self.0.borrow().clone()
    }
}

impl<K: Hash + Eq, V: Clone> MapCache<K, V> {
    /// Create a cache.
    pub fn new() -> MapCache<K, V> {
        MapCache(RefCell::new(HashMap::new()))
    }

    /// Ensure the value is present in the cache.
    pub fn ensure_value<F>(&self, key: K, default: F) -> ApiResult<()>
            where F: FnOnce(&K) -> ApiResult<V> {
        if self.0.borrow().contains_key(&key) {
            return Ok(());
        }

        let new = default(&key)?;
        let _ = self.0.borrow_mut().insert(key, new);
        Ok(())
    }

    /// Get a reference to the value.
    ///
    /// Borrows the inner RefCell.
    pub fn get_ref(&self, key: &K) -> Option<Ref<V>> {
        let map = self.0.borrow();
        if map.contains_key(key) {
            Some(Ref::map(map, |m| m.get(&key).unwrap()))
        } else {
            None
        }
    }
}

/// Deserialize value where empty string equals None.
#[allow(dead_code)]
pub fn empty_as_none<'de, D, T>(des: D) -> Result<Option<T>, D::Error>
        where D: Deserializer<'de>, T: FromStr, T::Err: Display {
    let s = String::deserialize(des)?;
    if s.is_empty() {
        Ok(None)
    } else {
        T::from_str(&s).map(Some).map_err(DeserError::custom)
    }
}


pub mod url {
    //! Handy primitives for working with URLs.

    #![allow(dead_code)] // unused with --no-default-features

    use hyper::Url;

    #[inline]
    #[allow(unused_results)]
    pub fn is_root(url: &Url) -> bool {
        url.path_segments().unwrap().filter(|x| !x.is_empty()).next().is_none()
    }

    #[inline]
    #[allow(unused_results)]
    pub fn join(mut url: Url, other: &str) -> Url {
        url.path_segments_mut().unwrap().pop_if_empty().push(other);
        url
    }

    #[inline]
    #[allow(unused_results)]
    pub fn extend<I>(mut url: Url, segments: I) -> Url
            where I: IntoIterator, I::Item: AsRef<str> {
        url.path_segments_mut().unwrap().pop_if_empty().extend(segments);
        url
    }

    #[inline]
    #[allow(unused_results)]
    pub fn pop(mut url: Url, keep_slash: bool) -> Url {
        url.path_segments_mut().unwrap().pop_if_empty().pop();
        if keep_slash {
            url.path_segments_mut().unwrap().pop_if_empty().push("");
        }
        url
    }
}
