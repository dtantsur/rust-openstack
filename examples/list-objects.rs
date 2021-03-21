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

use std::env;

#[cfg(feature = "object-storage")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");

    let container_name = env::args().nth(1).expect("Provide a container name");
    let container = os.get_container(&container_name).expect("Cannot get a container");

    println!("Found container with Name = {}, Number of object = {}",
        container.name(),
        container.object_count()
    );

    let objects: Vec<openstack::object_storage::Object> = container
        .find_objects()
        .with_limit(10)
        .all()
        .expect("cannot list objects");

    println!("first 10 objects");
    for o in objects {
        println!("Name = {}, Bytes = {}, Hash = {}",
            o.name(),
            o.bytes(),
            o.hash(),
        );
    }
}

#[cfg(not(feature = "object-storage"))]
fn main() {
    panic!("This example cannot run with 'object-storage' feature disabled");
}
