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
extern crate fallible_iterator;
extern crate openstack;

use fallible_iterator::FallibleIterator;


#[cfg(feature = "compute")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::compute::ServerSortKey::AccessIpv4;

    let servers: Vec<openstack::compute::Server> = os.find_servers()
        .sort_by(openstack::Sort::Asc(sorting))
        .detailed().into_iter().take(10).collect()
        .expect("Cannot list servers");
    println!("First 10 servers:");
    for s in &servers {
        println!("ID = {}, Name = {}", s.id(), s.name());
    }

    let active = os.find_servers()
        .sort_by(openstack::Sort::Asc(sorting))
        .with_status(openstack::compute::ServerStatus::Active)
        .all().expect("Cannot list servers");
    println!("All active servers:");
    for s in &active {
        println!("ID = {}, Name = {}", s.id(), s.name());
    }
}

#[cfg(not(feature = "compute"))]
fn main() {
    panic!("This example cannot run with 'compute' feature disabled");
}
