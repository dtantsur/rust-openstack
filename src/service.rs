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

use std::cmp;
use std::fmt::Debug;
use std::time::Duration;
use std::vec;

use fallible_iterator::FallibleIterator;
use reqwest::Url;
use reqwest::header::Headers;
use serde::Serialize;

use super::{Error, ErrorKind, Result, ApiVersion, Waiter};
use super::auth::AuthMethod;
use super::session::{ApiVersionRequest, Session};
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

/// Trait representing a service with API version support.
pub trait ApiVersioning {}


impl ServiceInfo {
    /// Pick an API version.
    pub fn pick_api_version(&self, request: ApiVersionRequest)
            -> Option<ApiVersion> {
        match request {
            ApiVersionRequest::Minimum =>
                self.minimum_version,
            ApiVersionRequest::Latest =>
                self.current_version,
            ApiVersionRequest::LatestFrom(from, to) => {
                match (self.current_version, self.minimum_version) {
                    (Some(max), None) if max >= from && max <= to => Some(max),
                    (None, Some(min)) if min >= from && min <= to => Some(min),
                    (Some(max), Some(min)) if to >= min && from <= max =>
                        Some(cmp::min(max, to)),
                    _ => None
                }
            },
            ApiVersionRequest::Exact(req) => {
                self.current_version.and_then(|max| {
                    match self.minimum_version {
                        Some(min) if req >= min && req <= max => Some(req),
                        None if req == max => Some(req),
                        _ => None
                    }
                })
            },
            ApiVersionRequest::Choice(vec) => {
                if vec.is_empty() {
                    return None;
                }

                self.current_version.and_then(|max| {
                    match self.minimum_version {
                        Some(min) => vec.into_iter().filter(|x| {
                            *x >= min && *x <= max
                        }).max(),
                        None =>vec.into_iter().find(|x| *x == max)
                    }
                })
            }
        }
    }
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


#[cfg(test)]
pub mod test {
    use reqwest::Url;

    use super::super::ApiVersion;
    use super::super::session::ApiVersionRequest;
    use super::ServiceInfo;

    fn service_info(min: Option<u16>, max: Option<u16>) -> ServiceInfo {
        ServiceInfo {
            root_url: Url::parse("http://127.0.0.1").unwrap(),
            minimum_version: min.map(|x| ApiVersion(2, x)),
            current_version: max.map(|x| ApiVersion(2, x)),
        }
    }

    #[test]
    fn test_pick_version_exact() {
        let info = service_info(Some(1), Some(24));
        let version = ApiVersion(2, 22);
        let result = info.pick_api_version(ApiVersionRequest::Exact(version))
            .unwrap();
        assert_eq!(result, version);
    }

    #[test]
    fn test_pick_version_exact_mismatch() {
        let info = service_info(Some(1), Some(24));
        let version = ApiVersion(2, 25);
        let res1 = info.pick_api_version(ApiVersionRequest::Exact(version));
        assert!(res1.is_none());
        let version2 = ApiVersion(1, 11);
        let res2 = info.pick_api_version(ApiVersionRequest::Exact(version2));
        assert!(res2.is_none());
    }

    #[test]
    fn test_pick_version_exact_current_only() {
        let info = service_info(None, Some(24));
        let version = ApiVersion(2, 24);
        let result = info.pick_api_version(ApiVersionRequest::Exact(version))
            .unwrap();
        assert_eq!(result, version);
    }

    #[test]
    fn test_pick_version_exact_current_only_mismatch() {
        let info = service_info(None, Some(24));
        let version = ApiVersion(2, 22);
        let result = info.pick_api_version(ApiVersionRequest::Exact(version));
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_minimum() {
        let info = service_info(Some(1), Some(24));
        let result = info.pick_api_version(ApiVersionRequest::Minimum)
            .unwrap();
        assert_eq!(result, ApiVersion(2, 1));
    }

    #[test]
    fn test_pick_version_minimum_unknown() {
        let info = service_info(None, Some(24));
        let result = info.pick_api_version(ApiVersionRequest::Minimum);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_latest() {
        let info = service_info(Some(1), Some(24));
        let result = info.pick_api_version(ApiVersionRequest::Latest)
            .unwrap();
        assert_eq!(result, ApiVersion(2, 24));
    }

    #[test]
    fn test_pick_version_latest_unknown() {
        let info = service_info(Some(1), None);
        let result = info.pick_api_version(ApiVersionRequest::Latest);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_latest_from_within_range() {
        let info = service_info(Some(1), Some(24));
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 5),
                                                ApiVersion(2, 20));
        let result = info.pick_api_version(req).unwrap();
        assert_eq!(result, ApiVersion(2, 20));
    }

    #[test]
    fn test_pick_version_latest_from_outside_range() {
        let info = service_info(Some(1), Some(24));
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 0),
                                                ApiVersion(2, 50));
        let result = info.pick_api_version(req).unwrap();
        assert_eq!(result, ApiVersion(2, 24));
    }

    #[test]
    fn test_pick_version_latest_from_mismatch_above() {
        let info = service_info(Some(1), Some(24));
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 25),
                                                ApiVersion(2, 50));
        let result = info.pick_api_version(req);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_latest_from_mismatch_below() {
        let info = service_info(Some(5), Some(24));
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 1),
                                                ApiVersion(2, 4));
        let result = info.pick_api_version(req);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_latest_from_only_current() {
        let info = service_info(None, Some(24));
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 5),
                                                ApiVersion(2, 50));
        let result = info.pick_api_version(req).unwrap();
        assert_eq!(result, ApiVersion(2, 24));
    }

    #[test]
    fn test_pick_version_latest_from_only_current_mismatch() {
        let info = service_info(None, Some(24));
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 5),
                                                ApiVersion(2, 10));
        let result = info.pick_api_version(req);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_latest_from_only_minimum() {
        let info = service_info(Some(1), None);
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 0),
                                                ApiVersion(2, 50));
        let result = info.pick_api_version(req).unwrap();
        assert_eq!(result, ApiVersion(2, 1));
    }

    #[test]
    fn test_pick_version_latest_from_only_minimum_mismatch() {
        let info = service_info(Some(1), None);
        let req = ApiVersionRequest::LatestFrom(ApiVersion(2, 5),
                                                ApiVersion(2, 10));
        let result = info.pick_api_version(req);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_choice() {
        let info = service_info(Some(1), Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 2),
                          ApiVersion(2, 22), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice))
            .unwrap();
        assert_eq!(result, ApiVersion(2, 22));
    }

    #[test]
    fn test_pick_version_choice_mismatch() {
        let info = service_info(Some(1), Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice));
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_choice_current_only() {
        let info = service_info(None, Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 2),
                          ApiVersion(2, 24), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice))
            .unwrap();
        assert_eq!(result, ApiVersion(2, 24));
    }

    #[test]
    fn test_pick_version_choice_current_only_mismatch() {
        let info = service_info(None, Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 2),
                          ApiVersion(2, 22), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice));
        assert!(result.is_none());
    }
}
