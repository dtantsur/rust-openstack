// Copyright 2018 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Foundation bits exposing the Network API.

use std::fmt::Debug;

use reqwest::{Method, Url};
use serde::Serialize;

use super::super::Result;
use super::super::auth::AuthMethod;
use super::super::common;
use super::super::session::{Session, ServiceInfo, ServiceType};
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Get a network.
    fn get_network<S: AsRef<str>>(&self, id: S) -> Result<protocol::Network>;

    /// List networks.
    fn list_networks<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Network>>;
}


/// Service type of Network API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "network";
const VERSION_ID: &'static str = "v2.0";


impl V2API for Session {
    fn get_network<S: AsRef<str>>(&self, id: S) -> Result<protocol::Network> {
        trace!("Get network {}", id.as_ref());
        let network = self.request::<V2>(Method::Get,
                                         &["networks", id.as_ref()],
                                         None)?
           .receive_json::<protocol::NetworkRoot>()?.network;
        trace!("Received {:?}", network);
        Ok(network)
    }

    fn list_networks<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Network>> {
        trace!("Listing networks with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["networks"], None)?
           .query(query).receive_json::<protocol::NetworksRoot>()?.networks;
        trace!("Received networks: {:?}", result);
        Ok(result)
    }
}


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        common::protocol::fetch_service_info(endpoint, auth, SERVICE_TYPE, VERSION_ID)
    }
}
