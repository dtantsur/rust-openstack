// Copyright 2019 Dmitry Tantsur <divius.inside@gmail.com>
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
extern crate ipnet;
extern crate openstack;

use std::io::{Cursor, Read};
use std::sync::{Once, ONCE_INIT};

use fallible_iterator::FallibleIterator;

use openstack::Refresh;

static INIT: Once = ONCE_INIT;

fn set_up() -> openstack::Cloud {
    INIT.call_once(|| {
        env_logger::init();
    });

    openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment")
}

#[test]
fn test_container_create() {
    let os = set_up();
    let name = "rust-openstack-integration-empty";

    let ctr = os
        .create_container(name)
        .expect("Failed to create a container");
    assert_eq!(ctr.name(), name);
    assert_eq!(ctr.bytes(), 0);
    assert_eq!(ctr.object_count(), 0);

    // Duplicate creation succeeds.
    let mut ctr2 = os
        .create_container(name)
        .expect("Failed to create a duplicate container");
    assert_eq!(ctr2.name(), name);
    assert_eq!(ctr2.bytes(), 0);
    assert_eq!(ctr2.object_count(), 0);

    ctr2.refresh().expect("Failed to refresh");
    assert_eq!(ctr2.name(), name);
    assert_eq!(ctr2.bytes(), 0);
    assert_eq!(ctr2.object_count(), 0);

    let found = os
        .find_containers()
        .into_iter()
        .find(|ctr| Ok(ctr.name() == name))
        .expect("Cannot list containers")
        .expect("Cannot find the container in listing");
    assert_eq!(found.name(), name);

    let found = os
        .find_containers()
        .with_prefix("rust-")
        .all()
        .expect("Failed to list containers with prefix");
    assert!(!found.is_empty());

    let found = os
        .find_containers()
        .with_prefix("definitely-not-rust")
        .all()
        .expect("Failed to list containers with prefix");
    assert!(found.is_empty());

    let objs = ctr.list_objects().expect("Failed to list objects");
    assert!(objs.is_empty());

    ctr.delete(false).expect("Failed to delete container");

    let found = os
        .find_containers()
        .into_iter()
        .find(|ctr| Ok(ctr.name() == name))
        .expect("Cannot list containers");
    assert!(found.is_none());
}

#[test]
fn test_object_create() {
    let os = set_up();
    let name = "rust-openstack-integration-1";

    let mut ctr = os
        .create_container(name)
        .expect("Failed to create a container");

    let buf = Cursor::new(vec![1, 2, 3, 4, 5]);

    let mut obj = os
        .create_object(ctr.clone(), "test1", buf)
        .expect("Failed to create an object");
    assert_eq!(obj.name(), "test1");
    assert_eq!(obj.container_name(), name);
    assert_eq!(obj.bytes(), 5);

    ctr.refresh().expect("Failed to refresh container");
    assert!(ctr.object_count() > 0);

    {
        let mut rdr = obj.download().expect("Failed to open download");
        let mut res = Vec::new();
        rdr.read_to_end(&mut res).expect("Failed to read object");
        assert_eq!(res, vec![1, 2, 3, 4, 5]);
    }

    let found = ctr
        .find_objects()
        .into_iter()
        .find(|obj| Ok(obj.name() == "test1"))
        .expect("Failed to find objects")
        .expect("Object was not found");
    assert_eq!(found.name(), obj.name());

    obj.refresh().expect("Failed to refresh object");

    obj.delete().expect("Failed to delete the object");

    let found = ctr
        .find_objects()
        .into_iter()
        .find(|obj| Ok(obj.name() == "test1"))
        .expect("Failed to find objects");
    assert!(found.is_none());

    ctr.delete(false).expect("Failed to delete the container");
}

#[test]
fn test_container_purge() {
    let os = set_up();
    let name = "rust-openstack-integration-2";

    let ctr = os
        .create_container(name)
        .expect("Failed to create a container");

    let buf = Cursor::new(vec![1, 2, 3, 4, 5]);

    let _ = os
        .create_object(ctr.clone(), "test1", buf)
        .expect("Failed to create an object");

    ctr.delete(true).expect("Failed to delete the container");

    let found = os
        .find_containers()
        .into_iter()
        .find(|ctr| Ok(ctr.name() == name))
        .expect("Cannot list containers");
    assert!(found.is_none());
}
