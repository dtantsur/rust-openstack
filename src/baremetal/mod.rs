// Copyright 2023 Dmitry Tantsur <dtantsur@protonmail.com>
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

//! Bare Metal API implementation bits.
//!
//! # Limitations
//!
//! This module requires Bare Metal API version 1.46 (Rocky) or newer.

mod api;
mod constants;
mod infos;
mod nodes;
mod protocol;
mod types;

pub use infos::{DriverInfo, ImageChecksum, InstanceInfo, Properties};
pub use nodes::{Node, NodeQuery, NodeSummary};
pub use types::{
    CleanStep, DeployStep, Fault, NodeFilter, NodeSortKey, PowerState, ProvisionState,
    TargetPowerState, TargetProvisionState,
};
