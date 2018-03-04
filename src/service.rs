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

//! Generic API bits for implementing new services.

use std::fmt::Debug;
use std::time::Duration;
use std::vec;

use fallible_iterator::FallibleIterator;
use reqwest::Url;
use reqwest::header::Headers;
use serde::Serialize;

use super::{Error, ErrorKind, Result, ApiVersion, Waiter};
use super::auth::AuthMethod;
use super::session::Session;
use super::utils::Query;


/// Information about API endpoint.
#[derive(Clone, Debug)]
pub struct ServiceInfo {
    /// Root endpoint.
    pub root_url: Url,
    /// Current API version (if supported).
    pub current_version: Option<ApiVersion>,
    /// Minimum API version (if supported).
    pub minimum_version: Option<ApiVersion>
}

/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Get basic service information.
    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo>;

    /// Return headers to set for this API version.
    fn api_version_headers(_version: ApiVersion) -> Option<Headers> { None }
}


/// Trait representing something that can be refreshed.
pub trait Refresh {
    /// Refresh the resource representation.
    fn refresh(&mut self) -> Result<()>;
}

/// Trait representing something that has an ID.
pub trait ResourceId {
    /// Identifier of the current resource.
    fn resource_id(&self) -> String;
}

/// Trait representing something that can be listed from a session.
pub trait ListResources<'a> {
    /// Default limit to use with this resource.
    const DEFAULT_LIMIT: usize;

    /// List the resources from the session.
    fn list_resources<Q: Serialize + Debug>(session: &'a Session, query: Q)
        -> Result<Vec<Self>> where Self: Sized;
}

/// Generic implementation of a `FallibleIterator` over resources.
#[derive(Debug, Clone)]
pub struct ResourceIterator<'session, T> {
    session: &'session Session,
    query: Query,
    cache: Option<vec::IntoIter<T>>,
    marker: Option<String>,
    can_paginate: bool,
}

/// Wait for resource deletion.
#[derive(Debug)]
pub struct DeletionWaiter<T> {
    inner: T,
    wait_timeout: Duration,
    delay: Duration,
}

impl<T> DeletionWaiter<T> {
    #[allow(dead_code)]  // unused with --no-default-features
    pub(crate) fn new(inner: T, wait_timeout: Duration, delay: Duration)
            -> DeletionWaiter<T> {
        DeletionWaiter {
            inner: inner,
            wait_timeout: wait_timeout,
            delay: delay,
        }
    }
}

impl<T: ResourceId + Refresh> Waiter<()> for DeletionWaiter<T> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        Some(self.wait_timeout)
    }

    fn default_delay(&self) -> Duration {
        self.delay
    }

    fn timeout_error_message(&self) -> String {
        format!("Timeout waiting for resource {} to be deleted",
                self.inner.resource_id())
    }

    fn poll(&mut self) -> Result<Option<()>> {
        match self.inner.refresh() {
            Ok(..) => {
                trace!("Still waiting for resource {} to be deleted",
                       self.inner.resource_id());
                Ok(None)
            },
            Err(ref e) if e.kind() == ErrorKind::ResourceNotFound => {
                debug!("Resource {} was deleted", self.inner.resource_id());
                Ok(Some(()))
            },
            Err(e) => {
                debug!("Failed to delete resource {} - {}",
                       self.inner.resource_id(), e);
                Err(e)
            }
        }
    }
}

impl<'session, T> ResourceIterator<'session, T> {
    #[allow(dead_code)]  // unused with --no-default-features
    pub(crate) fn new(session: &'session Session, query: Query)
            -> ResourceIterator<'session, T> {
        let can_paginate = query.0.iter().all(|pair| {
            pair.0 != "limit" && pair.0 != "marker"
        });

        ResourceIterator {
            session: session,
            query: query,
            cache: None,
            marker: None,
            can_paginate: can_paginate
        }
    }
}

impl<'session, T> FallibleIterator for ResourceIterator<'session, T>
        where T: ListResources<'session> + ResourceId {
    type Item = T;

    type Error = Error;

    fn next(&mut self) -> Result<Option<T>> {
        let maybe_next = self.cache.as_mut().and_then(|cache| cache.next());
        Ok(if maybe_next.is_some() {
            maybe_next
        } else {
            if self.cache.is_some() && ! self.can_paginate {
                // We have exhausted the results and pagination is not possible
                None
            } else {
                let mut query = self.query.clone();

                if self.can_paginate {
                    // can_paginate=true implies no limit was provided
                    query.push("limit", T::DEFAULT_LIMIT);
                    if let Some(marker) = self.marker.take() {
                        query.push_str("marker", marker);
                    }
                }

                let mut servers_iter = T::list_resources(self.session,
                                                         &query.0)?
                    .into_iter();
                let maybe_next = servers_iter.next();
                self.cache = Some(servers_iter);

                maybe_next
            }
        }.map(|next| {
            self.marker = Some(next.resource_id());
            next
        }))
    }
}
