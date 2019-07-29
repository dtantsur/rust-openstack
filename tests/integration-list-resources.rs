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
extern crate fallible_iterator;
extern crate openstack;

use std::sync::{Once, ONCE_INIT};

static INIT: Once = ONCE_INIT;

fn set_up() -> openstack::Cloud {
    INIT.call_once(|| {
        env_logger::init();
    });

    openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment")
}

#[test]
fn test_list_containers() {
    let os = set_up();
    let _ = os.list_containers().expect("Cannot list containers");
}

#[test]
fn test_list_flavors() {
    let os = set_up();
    let items = os.list_flavors().expect("Cannot list flavors");
    assert!(!items.is_empty());
}

#[test]
fn test_list_images() {
    let os = set_up();
    let items = os.list_images().expect("Cannot list images");
    assert!(!items.is_empty());
}

#[test]
fn test_list_keypairs() {
    let os = set_up();
    let _ = os.list_keypairs().expect("Cannot list key pairs");
}

#[test]
fn test_list_networks() {
    let os = set_up();
    let items = os.list_networks().expect("Cannot list networks");
    assert!(!items.is_empty());
}

#[test]
fn test_list_ports() {
    let os = set_up();
    let _ = os.list_ports().expect("Cannot list ports");
}

#[test]
fn test_list_servers() {
    let os = set_up();
    let _ = os.list_servers().expect("Cannot list servers");
}

#[test]
fn test_list_subnets() {
    let os = set_up();
    let _ = os.list_subnets().expect("Cannot list subnets");
}
