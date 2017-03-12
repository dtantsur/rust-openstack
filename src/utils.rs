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

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;

use hyper::Client;
#[cfg(feature = "tls")]
use hyper::net::HttpsConnector;
#[cfg(feature = "tls")]
use hyper_rustls::TlsClient;
use uuid::Uuid;

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
#[derive(Debug)]
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

        let new = try!(default());

        *self.0.borrow_mut() = Some(new);
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

        let new = try!(default(&key));

        let _ = self.0.borrow_mut().insert(key, new);
        Ok(())
    }

    /// Get the cached value.
    #[inline]
    pub fn get(&self, key: &K) -> Option<V> {
        self.0.borrow().get(key).cloned()
    }
}


/// Something that can be converted to an ID.
pub trait IntoId {
    /// Convert a value into an ID.
    fn into_id(self) -> String;
}

impl IntoId for Uuid {
    fn into_id(self) -> String {
        self.to_string()
    }
}

impl IntoId for String {
    fn into_id(self) -> String {
        self
    }
}

impl<'a> IntoId for &'a String {
    fn into_id(self) -> String {
        self.clone()
    }
}

impl<'a> IntoId for &'a str {
    fn into_id(self) -> String {
        String::from(self)
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
    pub fn extend(mut url: Url, segments: &[&str]) -> Url {
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
