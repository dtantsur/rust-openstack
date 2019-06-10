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

#![allow(dead_code)] // various things are unused with --no-default-features

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

use serde::{Serialize, Serializer};

use super::{Error, ErrorKind, Result};

/// Type of query parameters.
#[derive(Clone)]
pub struct Query(pub Vec<(String, String)>);

/// Cached clone-able value.
#[derive(Debug, Clone)]
pub struct ValueCache<T: Clone>(RefCell<Option<T>>);

/// Cached map of values.
#[derive(Debug, Clone)]
pub struct MapCache<K: Hash + Eq, V: Clone>(RefCell<HashMap<K, V>>);

impl fmt::Debug for Query {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(f, "{:?}", self.0)
    }
}

impl Query {
    /// Empty query.
    pub fn new() -> Query {
        Query(Vec::new())
    }

    /// Add an item to the query.
    #[allow(clippy::needless_pass_by_value)] // TODO: fix
    pub fn push<K, V>(&mut self, param: K, value: V)
    where
        K: Into<String>,
        V: ToString,
    {
        self.0.push((param.into(), value.to_string()))
    }

    /// Add a strng item to the query.
    pub fn push_str<K, V>(&mut self, param: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.0.push((param.into(), value.into()))
    }

    /// Add marker and limit to the query and clone it.
    pub fn with_marker_and_limit(&self, limit: Option<usize>, marker: Option<String>) -> Query {
        let mut new = self.clone();
        if let Some(limit_) = limit {
            new.push("limit", limit_);
        }
        if let Some(marker_) = marker {
            new.push_str("marker", marker_);
        }
        new
    }
}

impl Serialize for Query {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<T: Clone> ValueCache<T> {
    /// Create a cache.
    pub fn new(value: Option<T>) -> ValueCache<T> {
        ValueCache(RefCell::new(value))
    }

    /// Ensure the value is cached.
    pub fn ensure_value<F>(&self, default: F) -> Result<()>
    where
        F: FnOnce() -> Result<T>,
    {
        if self.0.borrow().is_some() {
            return Ok(());
        };

        *self.0.borrow_mut() = Some(default()?);
        Ok(())
    }

    /// Ensure that the cached value is valid.
    ///
    /// Returns `true` if the value exists and passes the check.
    pub fn validate<F>(&self, check: F) -> bool
    where
        F: FnOnce(&T) -> bool,
    {
        let valid = match self.0.borrow().as_ref() {
            Some(v) => check(v),
            None => false,
        };

        if !valid {
            *self.0.borrow_mut() = None;
            false
        } else {
            true
        }
    }

    /// Validate value and set it if it is not valid.
    pub fn validate_and_ensure_value<V, F>(&self, check: V, default: F) -> Result<()>
    where
        V: FnOnce(&T) -> bool,
        F: FnOnce() -> Result<T>,
    {
        if self.validate(check) {
            Ok(())
        } else {
            self.ensure_value(default)
        }
    }

    /// Extract a part of the value.
    pub fn extract<F, R>(&self, filter: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        self.0.borrow().as_ref().map(filter)
    }
}

impl<K: Hash + Eq, V: Clone> MapCache<K, V> {
    /// Create a cache.
    pub fn new() -> MapCache<K, V> {
        MapCache(RefCell::new(HashMap::new()))
    }

    /// Ensure the value is present in the cache.
    pub fn ensure_value<F>(&self, key: K, default: F) -> Result<()>
    where
        F: FnOnce(&K) -> Result<V>,
    {
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

/// Extensions for Result type.
pub trait ResultExt<T> {
    /// Process result if the error was ResourceNotFound.
    fn if_not_found_then<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn if_not_found_then<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        self.or_else(|err| {
            if err.kind() == ErrorKind::ResourceNotFound {
                f()
            } else {
                Err(err)
            }
        })
    }
}

/// Get one and only one item from an iterator.
pub fn one<T, I, S>(collection: I, not_found_msg: S, too_many_msg: S) -> Result<T>
where
    I: IntoIterator<Item = T>,
    S: Into<String>,
{
    let mut iter = collection.into_iter();
    let result = iter
        .next()
        .ok_or_else(|| Error::new(ErrorKind::ResourceNotFound, not_found_msg.into()))?;

    if iter.next().is_some() {
        Err(Error::new(ErrorKind::TooManyItems, too_many_msg.into()))
    } else {
        Ok(result)
    }
}

pub fn endpoint_not_found<D: fmt::Display>(service_type: D) -> Error {
    Error::new(
        ErrorKind::EndpointNotFound,
        format!("Endpoint for service {} was not found", service_type),
    )
}

pub mod url {
    //! Handy primitives for working with URLs.

    use reqwest::Url;

    #[inline]
    #[allow(unused_results)]
    pub fn is_root(url: &Url) -> bool {
        url.path_segments().unwrap().any(|x| !x.is_empty())
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
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        url.path_segments_mut()
            .unwrap()
            .pop_if_empty()
            .extend(segments);
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
