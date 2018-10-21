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

use super::super::Result;
use super::super::auth::AuthMethod;
use super::super::common::{self, ApiVersion};
use super::super::common::protocol::Ref;
use super::super::session::{Session, ServiceInfo, ServiceType};
use super::super::utils::{self, ResultExt};
use super::protocol;


const API_VERSION_KEYPAIR_TYPE: ApiVersion = ApiVersion(2, 2);
const API_VERSION_SERVER_DESCRIPTION: ApiVersion = ApiVersion(2, 19);
const API_VERSION_KEYPAIR_PAGINATION: ApiVersion = ApiVersion(2, 35);
const API_VERSION_FLAVOR_DESCRIPTION: ApiVersion = ApiVersion(2, 55);
const API_VERSION_FLAVOR_EXTRA_SPECS: ApiVersion = ApiVersion(2, 61);


/// Extensions for Session.
pub trait V2API {
    /// Create a key pair.
    fn create_keypair(&self, request: protocol::KeyPairCreate) -> Result<protocol::KeyPair>;

    /// Create a server.
    fn create_server(&self, request: protocol::ServerCreate) -> Result<Ref>;

    /// Delete a key pair.
    fn delete_keypair<S: AsRef<str>>(&self, name: S) -> Result<()>;

    /// Delete a server.
    fn delete_server<S: AsRef<str>>(&self, id: S) -> Result<()>;

    /// Get a flavor by its ID.
    fn get_extra_specs_by_flavor_id<S: AsRef<str>>(&self, id: S)
        -> Result<HashMap<String, String>>;

    /// Get a flavor.
    fn get_flavor<S: AsRef<str>>(&self, id_or_name: S) -> Result<protocol::Flavor> {
        let s = id_or_name.as_ref();
        self.get_flavor_by_id(s).if_not_found_then(|| self.get_flavor_by_name(s))
    }

    /// Get a flavor by its ID.
    fn get_flavor_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Flavor>;

    /// Get a flavor by its name.
    fn get_flavor_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Flavor>;

    /// Get a key pair by its nam.e
    fn get_keypair<S: AsRef<str>>(&self, name: S) -> Result<protocol::KeyPair>;

    /// Get a server.
    fn get_server<S: AsRef<str>>(&self, id_or_name: S) -> Result<protocol::Server> {
        let s = id_or_name.as_ref();
        self.get_server_by_id(s).if_not_found_then(|| self.get_server_by_name(s))
    }

    /// Get a server by its ID.
    fn get_server_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Server>;

    /// Get a server by its ID.
    fn get_server_by_name<S: AsRef<str>>(&self, id: S) -> Result<protocol::Server>;

    /// List flavors.
    fn list_flavors<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<common::protocol::IdAndName>>;

    /// List flavors with details.
    fn list_flavors_detail<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Flavor>>;

    /// List key pairs.
    fn list_keypairs<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::KeyPair>>;

    /// List servers.
    fn list_servers<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<common::protocol::IdAndName>>;

    /// List servers with details.
    fn list_servers_detail<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Server>>;

    /// Pick the highest API version or None if neither is supported.
    fn pick_compute_api_version(&self, versions: &[ApiVersion]) -> Result<Option<ApiVersion>>;

    /// Run an action while providing some arguments.
    fn server_action_with_args<S1, S2, Q>(&self, id: S1, action: S2, args: Q)
        -> Result<()> where S1: AsRef<str>, S2: AsRef<str>, Q: Serialize + Debug;

    /// Run an action on the server.
    fn server_simple_action<S1, S2>(&self, id: S1, action: S2) -> Result<()>
            where S1: AsRef<str>, S2: AsRef<str> {
        self.server_action_with_args(id, action, serde_json::Value::Null)
    }

    /// Whether the given compute API version is supported by the server.
    fn supports_compute_api_version(&self, version: ApiVersion) -> Result<bool>;

    /// Whether key pair pagination is supported.
    fn supports_keypair_pagination(&self) -> Result<bool> {
        self.supports_compute_api_version(API_VERSION_KEYPAIR_PAGINATION)
    }
}

/// Service type of Compute API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &str = "compute";

fn flavor_api_version<T: V2API>(api: &T) -> Result<Option<ApiVersion>> {
    api.pick_compute_api_version(
        &[API_VERSION_FLAVOR_DESCRIPTION,
          API_VERSION_FLAVOR_EXTRA_SPECS]
    )
}

impl V2API for Session {
    fn create_keypair(&self, request: protocol::KeyPairCreate)
            -> Result<protocol::KeyPair> {
        debug!("Creating a key pair with {:?}", request);
        let body = protocol::KeyPairCreateRoot { keypair: request };
        let keypair = self.request::<V2>(Method::Post, &["os-keypairs"], None)?
            .json(&body).receive_json::<protocol::KeyPairRoot>()?.keypair;
        debug!("Created key pair {:?}", keypair);
        Ok(keypair)
    }

    fn create_server(&self, request: protocol::ServerCreate) -> Result<Ref> {
        debug!("Creating a server with {:?}", request);
        let body = protocol::ServerCreateRoot { server: request };
        let server = self.request::<V2>(Method::Post, &["servers"], None)?
            .json(&body).receive_json::<protocol::CreatedServerRoot>()?.server;
        trace!("Requested creation of server {:?}", server);
        Ok(server)
    }

    fn delete_keypair<S: AsRef<str>>(&self, name: S) -> Result<()> {
        debug!("Deleting key pair {}", name.as_ref());
        let _ = self.request::<V2>(Method::Delete,
                                   &["os-keypairs", name.as_ref()],
                                   None)?
            .send()?;
        debug!("Key pair {} was deleted", name.as_ref());
        Ok(())
    }

    fn delete_server<S: AsRef<str>>(&self, id: S) -> Result<()> {
        trace!("Deleting server {}", id.as_ref());
        let _ = self.request::<V2>(Method::Delete,
                                   &["servers", id.as_ref()],
                                   None)?
            .send()?;
        debug!("Successfully requested deletion of server {}", id.as_ref());
        Ok(())
    }

    fn get_extra_specs_by_flavor_id<S: AsRef<str>>(&self, id: S)
            -> Result<HashMap<String, String>> {
        trace!("Get compute extra specs by ID {}", id.as_ref());
        let extra_specs = self.request::<V2>(Method::Get,
                                             &["flavors", id.as_ref(),
                                               "os-extra_specs"],
                                             None)?
           .receive_json::<protocol::ExtraSpecsRoot>()?.extra_specs;
        trace!("Received {:?}", extra_specs);
        Ok(extra_specs)
    }

    fn get_flavor_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Flavor> {
        trace!("Get compute flavor by ID {}", id.as_ref());
        let version = flavor_api_version(self)?;
        let flavor = self.request::<V2>(Method::Get,
                                        &["flavors", id.as_ref()],
                                        version)?
           .receive_json::<protocol::FlavorRoot>()?.flavor;
        trace!("Received {:?}", flavor);
        Ok(flavor)
    }

    fn get_flavor_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Flavor> {
        trace!("Get compute flavor by name {}", name.as_ref());
        let items = self.request::<V2>(Method::Get, &["flavors"], None)?
            .receive_json::<protocol::FlavorsRoot>()?.flavors
            .into_iter().filter(|item| item.name == name.as_ref());
        utils::one(items, "Flavor with given name or ID not found",
                   "Too many flavors found with given name")
            .and_then(|item| self.get_flavor_by_id(item.id))
    }

    fn get_keypair<S: AsRef<str>>(&self, name: S) -> Result<protocol::KeyPair> {
        trace!("Get compute key pair by name {}", name.as_ref());
        let ver = self.pick_compute_api_version(&[API_VERSION_KEYPAIR_TYPE])?;
        let keypair = self.request::<V2>(Method::Get,
                                        &["os-keypairs", name.as_ref()],
                                        ver)?
           .receive_json::<protocol::KeyPairRoot>()?.keypair;
        trace!("Received {:?}", keypair);
        Ok(keypair)
    }

    fn get_server_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Server> {
        trace!("Get compute server with ID {}", id.as_ref());
        let version = self.pick_compute_api_version(&[API_VERSION_SERVER_DESCRIPTION])?;
        let server = self.request::<V2>(Method::Get,
                                        &["servers", id.as_ref()],
                                        version)?
           .receive_json::<protocol::ServerRoot>()?.server;
        trace!("Received {:?}", server);
        Ok(server)
    }

    fn get_server_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Server> {
        trace!("Get compute server with name {}", name.as_ref());
        let items = self.request::<V2>(Method::Get, &["servers"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::ServersRoot>()?.servers
            .into_iter().filter(|item| item.name == name.as_ref());
        utils::one(items, "Server with given name or ID not found",
                   "Too many servers found with given name")
            .and_then(|item| self.get_server_by_id(item.id))
    }

    fn list_flavors<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<common::protocol::IdAndName>> {
        trace!("Listing compute flavors with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["flavors"], None)?
           .query(query).receive_json::<protocol::FlavorsRoot>()?.flavors;
        trace!("Received flavors: {:?}", result);
        Ok(result)
    }

    fn list_flavors_detail<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Flavor>> {
        trace!("Listing compute flavors with {:?}", query);
        let version = self.pick_compute_api_version(&[API_VERSION_FLAVOR_EXTRA_SPECS])?;
        let result = self.request::<V2>(Method::Get,
                                        &["flavors", "detail"],
                                        version)?
           .query(query).receive_json::<protocol::FlavorsDetailRoot>()?.flavors;
        trace!("Received flavors: {:?}", result);
        Ok(result)
    }

    fn list_keypairs<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::KeyPair>> {
        trace!("Listing compute key pairs with {:?}", query);
        let ver = self.pick_compute_api_version(&[API_VERSION_KEYPAIR_TYPE,
                                                  API_VERSION_KEYPAIR_PAGINATION])?;
        let result = self.request::<V2>(Method::Get, &["os-keypairs"], ver)?
           .query(query).receive_json::<protocol::KeyPairsRoot>()?.keypairs
           .into_iter().map(|item| item.keypair).collect::<Vec<_>>();
        trace!("Received key pairs: {:?}", result);
        Ok(result)
    }

    fn list_servers<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<common::protocol::IdAndName>> {
        trace!("Listing compute servers with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["servers"], None)?
           .query(query).receive_json::<protocol::ServersRoot>()?.servers;
        trace!("Received servers: {:?}", result);
        Ok(result)
    }

    fn list_servers_detail<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Server>> {
        trace!("Listing compute servers with {:?}", query);
        let version = self.pick_compute_api_version(&[API_VERSION_SERVER_DESCRIPTION])?;
        let result = self.request::<V2>(Method::Get,
                                        &["servers", "detail"],
                                        version)?
           .query(query).receive_json::<protocol::ServersDetailRoot>()?.servers;
        trace!("Received servers: {:?}", result);
        Ok(result)
    }

    fn pick_compute_api_version(&self, versions: &[ApiVersion]) -> Result<Option<ApiVersion>> {
        let info = self.get_service_info_ref::<V2>()?;
        Ok(versions.into_iter().filter(|item| {
            info.supports_api_version(**item)
        }).max().cloned())
    }

    fn server_action_with_args<S1, S2, Q>(&self, id: S1, action: S2, args: Q)
            -> Result<()>
            where S1: AsRef<str>, S2: AsRef<str>, Q: Serialize + Debug {
        trace!("Running {} on server {} with args {:?}",
               action.as_ref(), id.as_ref(), args);
        let mut body = HashMap::new();
        let _ = body.insert(action.as_ref(), args);
        let _ = self.request::<V2>(Method::Post,
                                   &["servers", id.as_ref(), "action"],
                                   None)?
            .json(&body).send()?;
        debug!("Successfully ran {} on server {}", action.as_ref(), id.as_ref());
        Ok(())
    }

    fn supports_compute_api_version(&self, version: ApiVersion) -> Result<bool> {
        let info = self.get_service_info_ref::<V2>()?;
        Ok(info.supports_api_version(version))
    }
}


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        common::protocol::fetch_service_info(endpoint, auth, SERVICE_TYPE,
                                             |ver| ver.0 == 2)
    }

    fn api_version_headers(version: ApiVersion) -> Option<Headers> {
        let mut hdrs = Headers::new();
        // TODO: typed header, new-style header support
        hdrs.set_raw("x-openstack-nova-api-version", version.to_string());
        Some(hdrs)
    }
}
