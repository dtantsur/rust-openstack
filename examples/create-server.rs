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

extern crate env_logger;
extern crate openstack;
extern crate waiter;

use std::env;
use waiter::{Waiter, WaiterCurrentState};


#[cfg(feature = "compute")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");

    let name = env::args().nth(1).expect("Provide a server name");
    let flavor = env::args().nth(2).expect("Provide a flavor");
    let image = env::args().nth(3).expect("Provide an image");
    let network = env::args().nth(4).expect("Provide a network");
    let keypair = env::args().nth(5).expect("Provide a key pair");

    let waiter = os.new_server(name, flavor)
        .with_image(image).with_network(network).with_keypair(keypair)
        .with_metadata("key", "value")
        .create().expect("Cannot create a server");
    {
        let current = waiter.waiter_current_state();
        println!("ID = {}, Name = {}, Status = {:?}, Power = {:?}",
                 current.id(), current.name(),
                 current.status(), current.power_state());
    }

    let server = waiter.wait().expect("Server did not reach ACTIVE");
    println!("ID = {}, Name = {}, Status = {:?}, Power = {:?}",
             server.id(), server.name(),
             server.status(), server.power_state());
}

#[cfg(not(feature = "compute"))]
fn main() {
    panic!("This example cannot run with 'compute' feature disabled");
}

