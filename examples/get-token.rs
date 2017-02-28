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

extern crate hyper;
extern crate openstack;

use openstack::auth::{AuthMethod, self};
use openstack::utils;


fn main() {
    let identity = auth::Identity::from_env()
        .expect("Failed to create an identity provider from the environment");
    let http_client = utils::http_client();
    let token_info = identity.get_token(&http_client)
        .expect("Failed to get a token");
    println!("Received token: {}", token_info.token);
}
