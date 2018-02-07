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

//! Foundation bits exposing the Compute API.

use std::fmt::Debug;

use reqwest::{Method, Url};
use reqwest::header::Headers;
use serde::Serialize;

use super::super::super::{Result, ApiVersion};
use super::super::super::auth::AuthMethod;
use super::super::super::common;
use super::super::super::service::{ApiVersioning, ServiceInfo, ServiceType};
use super::super::super::session::Session;
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Get a server.
    fn get_server<S: AsRef<str>>(&self, id: S) -> Result<protocol::Server>;

    /// List servers.
    fn list_servers<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::ServerSummary>>;

    /// List servers with details.
    fn list_servers_detail<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Server>>;
}

/// Service type of Compute API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "compute";
const VERSION_ID: &'static str = "v2.1";

impl V2API for Session {
    fn get_server<S: AsRef<str>>(&self, id: S) -> Result<protocol::Server> {
        trace!("Get compute server {}", id.as_ref());
        let server = self.request::<V2>(Method::Get, &["servers", id.as_ref()])?
           .receive_json::<protocol::ServerRoot>()?.server;
        trace!("Received {:?}", server);
        Ok(server)
    }

    fn list_servers<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::ServerSummary>> {
        trace!("Listing compute servers with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["servers"])?
           .query(query).receive_json::<protocol::ServersRoot>()?.servers;
        trace!("Received servers: {:?}", result);
        Ok(result)
    }

    fn list_servers_detail<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Server>> {
        trace!("Listing compute servers with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["servers", "detail"])?
           .query(query).receive_json::<protocol::ServersDetailRoot>()?.servers;
        trace!("Received servers: {:?}", result);
        Ok(result)
    }
}


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        common::fetch_service_info(endpoint, auth, SERVICE_TYPE, VERSION_ID)
    }

    fn api_version_headers(version: ApiVersion) -> Option<Headers> {
        let mut hdrs = Headers::new();
        // TODO: typed header, new-style header support
        hdrs.set_raw("x-openstack-nova-api-version", version.to_string());
        Some(hdrs)
    }
}

impl ApiVersioning for V2 {}
