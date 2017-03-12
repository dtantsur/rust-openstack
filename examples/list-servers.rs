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

extern crate env_logger;
extern crate hyper;
extern crate openstack;


#[cfg(feature = "compute")]
fn main() {
    env_logger::init().unwrap();

    let identity = openstack::auth::Identity::from_env()
        .expect("Failed to create an identity provider from the environment");
    let session = openstack::Session::new(identity);
    let servers = openstack::compute::servers::manager(&session).list()
        .expect("Cannot list servers");
    for s in &servers {
        println!("ID = {}, Name = {}", s.id(), s.name());
    }
}

#[cfg(not(feature = "compute"))]
fn main() {
    panic!("This example cannot run with 'compute' feature disabled");
}
