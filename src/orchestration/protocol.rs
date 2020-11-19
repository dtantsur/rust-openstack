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

//! JSON structures and protocol bits for the Orchestration API.

#![allow(non_snake_case)]
#![allow(missing_docs)]

use chrono::{DateTime, FixedOffset};
use osproto::common::empty_as_default;
use serde::{Deserialize, Serialize};


protocol_enum! {
    #[doc = "Possible server statuses."]
    enum StackStatus {
        Complete = "CREATE_COMPLETE",
        Failed = "CREATE_COMPLETE",
        InProgress = "CREATE_IN_PROGRESS",
        Unknown = "UNKNOWN"
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Stack {
    #[serde(rename = "created")]
    pub created_at: DateTime<FixedOffset>,
    #[serde(deserialize_with = "empty_as_default", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub status: StackStatus,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackCreate {
    pub name: String,
}

impl Default for StackStatus {
    fn default() -> StackStatus {
        StackStatus::Unknown
    }
}
