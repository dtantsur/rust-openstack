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

//! JSON structures and protocol bits for the Compute API.

#![allow(missing_docs)]

use std::io::Read;

use serde_json;


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Server {
    pub id: String,
    pub name: String
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServersRoot {
    pub servers: Vec<Server>
}


impl ServersRoot {
    pub fn from_reader<R: Read>(reader: R)
            -> Result<ServersRoot, serde_json::Error> {
        serde_json::from_reader(reader)
    }
}
