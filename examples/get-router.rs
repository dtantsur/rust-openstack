// Copyright 2020 Martin Chlumsky <martin.chlumsky@gmail.com>
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

use std::env;

#[cfg(feature = "network")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let id_or_name = env::args().nth(1).expect("Provide a router ID or name");
    let router = os
        .get_router(id_or_name)
        .await
        .expect("Cannot get a router");

    println!(
        "ID = {}, Name = {:?}, UP = {}, description = {:?}, status = {:?}, external_gateway_info = {:?}, routes = {:?}, distributed = {:?}, ha = {:?}",
        router.id(),
        router.name(),
        router.admin_state_up(),
        router.description(),
        router.status(),
        router.external_gateway(),
        router.routes(),
        router.distributed(),
        router.ha()
    );
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
