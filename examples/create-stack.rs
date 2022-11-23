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

#[cfg(feature = "orchestration")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");

    let stackf = env::args().nth(1).expect("Provide a stack file");
    let envf = env::args().nth(2).expect("Provide an env file");

    let waiter = os
        .new_stack(stackf, envf)
        .create()
        .expect("Cannot create a server");
    {
        let current = waiter.waiter_current_state();
        println!(
            "ID = {}, Name = {}, Status = {:?}",
            current.id(),
            current.name(),
            current.status(),
        );
    }

    let server = waiter.wait().expect("Stack did not reach COMPLETE");
    println!(
        "ID = {}, Name = {}, Status = {:?}",
        server.id(),
        server.name(),
        server.status(),
    );
}

#[cfg(not(feature = "orchestration"))]
fn main() {
    panic!("This example cannot run with 'orchestration' feature disabled");
}
