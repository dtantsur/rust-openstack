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

use std::marker::PhantomData;

use hyper::{Get, Url};
use serde::Deserialize;
use serde_json;

use super::{ApiResult, Session};
use super::auth::Method as AuthMethod;
use super::utils;
pub use super::utils::IntoId;


/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Version suffix to append to the endpoint.
    fn version_suffix() -> Option<&'static str>;
}

/// Low-level API calls.
#[derive(Debug)]
pub struct ServiceApi<'a, Auth: AuthMethod + 'a, Service> {
    session: &'a Session<Auth>,
    service_type: PhantomData<Service>,
    endpoint_interface: Option<String>,
    cached_endpoint: utils::ValueCache<Url>
}


impl<'a, Auth: AuthMethod + 'a, S: ServiceType> ServiceApi<'a, Auth, S> {
    /// Create a new API instance using the given session.
    pub fn new(session: &'a Session<Auth>) -> ServiceApi<'a, Auth, S> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: None,
            cached_endpoint: utils::ValueCache::new(None)
        }
    }

    /// Create a new API instance using the given session.
    pub fn new_with_endpoint<S1>(session: &'a Session<Auth>,
                                 endpoint_interface: S1)
            -> ServiceApi<'a, Auth, S> where S1: Into<String> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: Some(endpoint_interface.into()),
            cached_endpoint: utils::ValueCache::new(None)
        }
    }

    /// Get the root endpoint with or without the major version.
    ///
    /// The resulting endpoint is cached on the current ServiceApi object.
    pub fn get_root_endpoint(&self, include_version: bool) -> ApiResult<Url> {
        try!(self.cached_endpoint.ensure_value(|| {
            match self.endpoint_interface {
                Some(ref s) => self.session.get_endpoint(S::catalog_type(),
                                                         s.clone()),
                None => self.session.get_default_endpoint(S::catalog_type())
            }
        }));

        let endpoint = self.cached_endpoint.get().unwrap();
        if include_version {
            if let Some(suffix) = S::version_suffix() {
                if !endpoint.path().ends_with(suffix) {
                    return endpoint.join(suffix).map_err(From::from);
                }
            }
        }

        Ok(endpoint)
    }

    /// Get an endpoint with version suffix and given path appended.
    pub fn get_endpoint(&self, path: &str) -> ApiResult<Url> {
        let endpoint = try!(self.get_root_endpoint(true));
        endpoint.join(path).map_err(From::from)
    }

    /// List entities.
    pub fn list<R: Deserialize>(&self, path: &str) -> ApiResult<R> {
        // TODO: filtering
        let url = try!(self.get_endpoint(path));
        debug!("Listing entities from {}", url);
        let resp = try!(self.session.request(Get, url).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }

    /// Get one entity.
    pub fn get<R: Deserialize, Id: IntoId>(&self, path: &str, id: Id)
            -> ApiResult<R> {
        // Url expects trailing /
        let root_path = if path.ends_with("/") {
            String::from(path)
        } else {
            format!("{}/", path)
        };
        let url = try!(self.get_endpoint(&root_path));
        let url_with_id = try!(url.join(&id.into_id()));
        debug!("Get one entity from {}", url);
        let resp = try!(self.session.request(Get, url_with_id).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }
}
