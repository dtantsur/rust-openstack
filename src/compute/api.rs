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

//! Low-level Compute API implementation.

use hyper;
use serde;
use serde_json;

use super::super::auth::AuthMethod;
use super::super::{ApiError, Session};

/// Low-level Compute API calls.
#[derive(Debug)]
pub struct ComputeApi<'a, A: AuthMethod + 'a> {
    session: &'a Session<A>,
    endpoint_interface: Option<String>,
    region: Option<String>
}

const SERVICE_TYPE: &'static str = "compute";

impl<'a, A: AuthMethod + 'a> ComputeApi<'a, A> {
    /// Create a new API instance using the given session.
    pub fn new(session: &'a Session<A>) -> ComputeApi<'a, A> {
        ComputeApi::new_with_endpoint_params(session, None, None)
    }

    /// Create a new API instance using the given session.
    ///
    /// This variant allows passing an endpoint type (defaults to public),
    /// and region (defaults to any).
    pub fn new_with_endpoint_params(session: &'a Session<A>,
                                    endpoint_interface: Option<&str>,
                                    region: Option<&str>)
            -> ComputeApi<'a, A> {
        ComputeApi {
            session: session,
            endpoint_interface: endpoint_interface.map(String::from),
            region: region.map(String::from)
        }
    }

    fn get_endpoint(&self, path: &str) -> Result<hyper::Url, ApiError> {
        // TODO: move this code to Session
        let endpoint = try!(self.session.get_endpoint(
                SERVICE_TYPE,
                self.endpoint_interface.as_ref().map(String::as_str),
                self.region.as_ref().map(String::as_str)));

        let with_version = if endpoint.path().ends_with("/v2.1") {
            endpoint
        } else {
            try!(endpoint.join("v2.1"))
        };

        with_version.join(path).map_err(From::from)
    }

    /// List servers.
    pub fn list<R: serde::Deserialize>(&self, path: &str)
            -> Result<R, ApiError> {
        let url = try!(self.get_endpoint(path));
        debug!("Listing entities from {}", url);
        let resp = try!(self.session.request(hyper::Get, url).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }
}
