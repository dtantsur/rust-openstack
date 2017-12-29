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
extern crate openstack;

const KILO: openstack::ApiVersion = openstack::ApiVersion(2, 3);
const LIBERTY: openstack::ApiVersion = openstack::ApiVersion(2, 12);
const MITAKA: openstack::ApiVersion = openstack::ApiVersion(2, 25);
const NEWTON: openstack::ApiVersion = openstack::ApiVersion(2, 38);
const OCATA: openstack::ApiVersion = openstack::ApiVersion(2, 42);

#[cfg(feature = "compute")]
fn main() {
    env_logger::init().unwrap();

    let identity = openstack::auth::Identity::from_env()
        .expect("Failed to create an identity provider from the environment");
    let mut session = openstack::Session::new(identity);

    let version_choice = vec![KILO, LIBERTY, MITAKA, NEWTON, OCATA];
    let version = session.negotiate_api_version::<openstack::compute::V2>(
        openstack::ApiVersionRequest::Choice(version_choice)
    ).expect("Unable to negotiation any Compute API version");

    match version {
        KILO => println!("Kilo API detected"),
        LIBERTY => println!("Liberty API detected"),
        MITAKA => println!("Mitaka API detected"),
        NEWTON => println!("Newton API detected"),
        OCATA => println!("Ocata API detected"),
        _ => unreachable!()
    }
    openstack::compute::v2::servers(&session).list()
        .expect(&format!("Cannot list servers with API version {}", version));
}

#[cfg(not(feature = "compute"))]
fn main() {
    panic!("This example cannot run with 'compute' feature disabled");
}


