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

//! Low-level code to work with the service catalog.

use super::super::utils;
use super::super::Result;
use super::protocol::{CatalogRecord, Endpoint};

/// Find an endpoint in the service catalog.
pub fn find_endpoint<'c>(
    catalog: &'c [CatalogRecord],
    service_type: &str,
    endpoint_interface: &str,
    region: &Option<String>,
) -> Result<&'c Endpoint> {
    let svc = match catalog.iter().find(|x| x.service_type == *service_type) {
        Some(s) => s,
        None => return Err(utils::endpoint_not_found(service_type)),
    };

    let maybe_endp: Option<&Endpoint>;
    if let Some(ref rgn) = *region {
        maybe_endp = svc
            .endpoints
            .iter()
            .find(|x| x.interface == *endpoint_interface && x.region == *rgn);
    } else {
        maybe_endp = svc
            .endpoints
            .iter()
            .find(|x| x.interface == *endpoint_interface);
    }

    maybe_endp.ok_or_else(|| utils::endpoint_not_found(service_type))
}

#[cfg(test)]
pub mod test {
    use super::super::super::{ErrorKind, Result};
    use super::super::protocol::{CatalogRecord, Endpoint};

    fn demo_service1() -> CatalogRecord {
        CatalogRecord {
            service_type: String::from("identity"),
            endpoints: vec![
                Endpoint {
                    interface: String::from("public"),
                    region: String::from("RegionOne"),
                    url: String::from("https://host.one/identity"),
                },
                Endpoint {
                    interface: String::from("internal"),
                    region: String::from("RegionOne"),
                    url: String::from("http://192.168.22.1/identity"),
                },
                Endpoint {
                    interface: String::from("public"),
                    region: String::from("RegionTwo"),
                    url: String::from("https://host.two:5000"),
                },
            ],
        }
    }

    fn demo_service2() -> CatalogRecord {
        CatalogRecord {
            service_type: String::from("baremetal"),
            endpoints: vec![
                Endpoint {
                    interface: String::from("public"),
                    region: String::from("RegionOne"),
                    url: String::from("https://host.one/baremetal"),
                },
                Endpoint {
                    interface: String::from("public"),
                    region: String::from("RegionTwo"),
                    url: String::from("https://host.two:6385"),
                },
            ],
        }
    }

    pub fn demo_catalog() -> Vec<CatalogRecord> {
        vec![demo_service1(), demo_service2()]
    }

    fn find_endpoint<'a>(
        cat: &'a Vec<CatalogRecord>,
        service_type: &str,
        interface_type: &str,
        region: Option<&str>,
    ) -> Result<&'a Endpoint> {
        super::find_endpoint(
            cat,
            &String::from(service_type),
            &String::from(interface_type),
            &region.map(String::from),
        )
    }

    #[test]
    fn test_find_endpoint() {
        let cat = demo_catalog();

        let e1 = find_endpoint(&cat, "identity", "public", None).unwrap();
        assert_eq!(&e1.url, "https://host.one/identity");

        let e2 = find_endpoint(&cat, "identity", "internal", None).unwrap();
        assert_eq!(&e2.url, "http://192.168.22.1/identity");

        let e3 = find_endpoint(&cat, "baremetal", "public", None).unwrap();
        assert_eq!(&e3.url, "https://host.one/baremetal");
    }

    #[test]
    fn test_find_endpoint_with_region() {
        let cat = demo_catalog();

        let e1 = find_endpoint(&cat, "identity", "public", Some("RegionTwo")).unwrap();
        assert_eq!(&e1.url, "https://host.two:5000");

        let e2 = find_endpoint(&cat, "identity", "internal", Some("RegionOne")).unwrap();
        assert_eq!(&e2.url, "http://192.168.22.1/identity");

        let e3 = find_endpoint(&cat, "baremetal", "public", Some("RegionTwo")).unwrap();
        assert_eq!(&e3.url, "https://host.two:6385");
    }

    fn assert_not_found(result: Result<&Endpoint>) {
        let err = result.err().unwrap();
        if err.kind() != ErrorKind::EndpointNotFound {
            panic!("Unexpected error {}", err);
        }
    }

    #[test]
    fn test_find_endpoint_not_found() {
        let cat = demo_catalog();

        assert_not_found(find_endpoint(&cat, "foobar", "public", None));
        assert_not_found(find_endpoint(&cat, "identity", "public", Some("RegionFoo")));
        assert_not_found(find_endpoint(&cat, "baremetal", "internal", None));
        assert_not_found(find_endpoint(
            &cat,
            "identity",
            "internal",
            Some("RegionTwo"),
        ));
    }
}
