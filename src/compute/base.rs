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

use std::collections::HashMap;
use std::fmt::Debug;

use reqwest::{Method, Url};
use reqwest::header::Headers;
use serde::Serialize;
use serde_json;

use super::super::{Result, ApiVersion};
use super::super::auth::AuthMethod;
use super::super::common;
use super::super::service::{ApiVersioning, ServiceInfo, ServiceType};
use super::super::session::Session;
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Get a server.
    fn get_server<S: AsRef<str>>(&self, id: S) -> Result<protocol::Server>;

    /// List servers.
    fn list_servers<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<common::protocol::IdAndName>>;

    /// List servers with details.
    fn list_servers_detail<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Server>>;

    /// Run an action on the server.
    fn server_simple_action<S1, S2>(&self, id: S1, action: S2) -> Result<()>
        where S1: AsRef<str>, S2: AsRef<str>;

    /// Delete a server.
    fn delete_server<S: AsRef<str>>(&self, id: S) -> Result<()>;

    /// Get a flavor.
    fn get_flavor<S: AsRef<str>>(&self, id: S) -> Result<protocol::Flavor>;

    /// List flavors.
    fn list_flavors<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<common::protocol::IdAndName>>;

    /// List flavors with details.
    fn list_flavors_detail<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Flavor>>;
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
            -> Result<Vec<common::protocol::IdAndName>> {
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

    fn server_simple_action<S1, S2>(&self, id: S1, action: S2) -> Result<()>
            where S1: AsRef<str>, S2: AsRef<str> {
        trace!("Running {} on server {}", action.as_ref(), id.as_ref());
        let mut body = HashMap::new();
        let _ = body.insert(action.as_ref(), serde_json::Value::Null);
        let _ = self.request::<V2>(Method::Post,
                                   &["servers", id.as_ref(), "action"])?
            .json(&body).send()?;
        debug!("Successfully ran {} on server {}", action.as_ref(), id.as_ref());
        Ok(())
    }

    fn delete_server<S: AsRef<str>>(&self, id: S) -> Result<()> {
        trace!("Deleting server {}", id.as_ref());
        let _ = self.request::<V2>(Method::Delete, &["servers", id.as_ref()])?
            .send()?;
        trace!("Successfully requested deletion of server {}", id.as_ref());
        Ok(())
    }

    fn get_flavor<S: AsRef<str>>(&self, id: S) -> Result<protocol::Flavor> {
        trace!("Get compute flavor {}", id.as_ref());
        let flavor = self.request::<V2>(Method::Get, &["flavors", id.as_ref()])?
           .receive_json::<protocol::FlavorRoot>()?.flavor;
        trace!("Received {:?}", flavor);
        Ok(flavor)
    }

    fn list_flavors<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<common::protocol::IdAndName>> {
        trace!("Listing compute flavors with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["flavors"])?
           .query(query).receive_json::<protocol::FlavorsRoot>()?.flavors;
        trace!("Received flavors: {:?}", result);
        Ok(result)
    }

    fn list_flavors_detail<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Flavor>> {
        trace!("Listing compute flavors with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["flavors", "detail"])?
           .query(query).receive_json::<protocol::FlavorsDetailRoot>()?.flavors;
        trace!("Received flavors: {:?}", result);
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
