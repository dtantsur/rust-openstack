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
use std::sync::{Once, ONCE_INIT};

use waiter::Waiter;


static INIT: Once = ONCE_INIT;

fn set_up() -> openstack::Cloud {
    INIT.call_once(|| { env_logger::init(); });

    openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment")
}


#[test]
fn test_basic_server_ops() {
    let os = set_up();
    let image_id = env::var("RUST_OPENSTACK_IMAGE").expect("Missing RUST_OPENSTACK_IMAGE");
    let flavor_id = env::var("RUST_OPENSTACK_FLAVOR").expect("Missing RUST_OPENSTACK_FLAVOR");
    let network_id = env::var("RUST_OPENSTACK_NETWORK").expect("Missing RUST_OPENSTACK_NETWORK");

    let mut server = os.new_server("rust-openstack-integration", flavor_id)
        .with_image(image_id).with_network(network_id)
        .create().expect("Failed to request server creation")
        .wait().expect("Server was not created");
    assert_eq!(server.name(), "rust-openstack-integration");
    assert_eq!(server.status(), openstack::compute::ServerStatus::Active);
    assert_eq!(server.power_state(), openstack::compute::ServerPowerState::Running);

    server.stop().expect("Failed to request power off")
        .wait().expect("Failed to power off");
    assert_eq!(server.power_state(), openstack::compute::ServerPowerState::Shutdown);

    server.start().expect("Failed to request power on")
        .wait().expect("Failed to power on");
    assert_eq!(server.power_state(), openstack::compute::ServerPowerState::Running);

    server.delete().expect("Failed to request deletion")
        .wait().expect("Failed to delete server");
}
