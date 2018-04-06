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

//! JSON structures and protocol bits for the Network API.

#![allow(non_snake_case)]
#![allow(missing_docs)]

use chrono::{DateTime, FixedOffset};

use super::super::common;


protocol_enum! {
    #[doc = "Possible network statuses."]
    enum NetworkStatus {
        Active = "ACTIVE",
        Down = "DOWN",
        Building = "BUILD",
        Error = "ERROR"
    }
}

protocol_enum! {
    #[doc = "Available sort keys."]
    enum NetworkSortKey {
        CreatedAt = "created_at",
        Id = "id",
        Name = "name",
        UpdatedAt = "updated_at"
    }
}

impl Default for NetworkSortKey {
    fn default() -> NetworkSortKey {
        NetworkSortKey::CreatedAt
    }
}

/// An network.
#[derive(Debug, Clone, Deserialize)]
pub struct Network {
    pub admin_state_up: bool,
    #[serde(default)]
    pub availability_zones: Vec<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "common::protocol::empty_as_none", default)]
    pub dns_domain: Option<String>,
    #[serde(rename = "router:external")]
    pub external: Option<bool>,
    pub id: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub mtu: Option<u32>,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub shared: bool,
    pub subnets: Vec<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<FixedOffset>>,
}

/// A network.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkRoot {
    pub network: Network
}

/// A list of networks.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworksRoot {
    pub networks: Vec<Network>
}
