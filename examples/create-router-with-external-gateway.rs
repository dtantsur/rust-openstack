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

    let name = env::args().nth(1).expect("Provide a router name");
    let external_network = env::args()
        .nth(2)
        .expect("Provide an external network name or ID");

    let external_gateway = openstack::network::ExternalGateway::new(external_network);

    let router = os
        .new_router()
        .with_name(name)
        .with_external_gateway(external_gateway)
        .create()
        .await
        .expect("Cannot create a router");

    println!(
        "ID = {}, Name = {}, Status = {:?}, external_gateway_info = {:?}",
        router.id(),
        router.name().as_ref().unwrap(),
        router.status(),
        router.external_gateway().as_ref().unwrap()
    );

    let _ = router
        .external_network()
        .await
        .expect("Cannot load external network");

    let _ = router.delete().await;
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
