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

use openstack::Refresh;

#[cfg(feature = "network")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let name = env::args().nth(1).expect("Provide a router name");

    let mut router = os
        .new_router()
        .with_name(name)
        .create()
        .await
        .expect("Cannot create a router");

    router.set_description("Updated description.");
    router
        .save()
        .await
        .expect("Failed to update router description.");
    router
        .refresh()
        .await
        .expect("Failed to refresh router object.");

    println!(
        "ID = {}, Name = {}, Status = {:?}, Description = {}",
        router.id(),
        router.name().as_ref().unwrap(),
        router.status(),
        router.description().as_ref().unwrap()
    );

    router.delete().await.expect("Failed to delete router.");
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
