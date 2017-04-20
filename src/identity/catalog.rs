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

use hyper::Url;

use super::super::{ApiError, ApiResult};
use super::super::utils;
use super::protocol::{CatalogRecord, Endpoint};


/// Build a catalog URI.
pub fn get_url(auth_url: Url) -> Url {
    utils::url::extend(auth_url, &["v3", "auth", "catalog"])
}

/// Find an endpoint in the service catalog.
pub fn find_endpoint(catalog: &Vec<CatalogRecord>, service_type: String,
                     endpoint_interface: String, region: Option<String>)
        -> ApiResult<&Endpoint> {
    let svc = match catalog.iter().find(|x| x.service_type == service_type) {
        Some(s) => s,
        None => return Err(ApiError::EndpointNotFound(service_type))
    };

    let maybe_endp: Option<&Endpoint>;
    if let Some(rgn) = region {
        maybe_endp = svc.endpoints.iter().find(
            |x| x.interface == endpoint_interface && x.region == rgn);
    } else {
        maybe_endp = svc.endpoints.iter().find(
            |x| x.interface == endpoint_interface);
    }

    maybe_endp.ok_or(ApiError::EndpointNotFound(service_type))
}


#[cfg(test)]
pub mod test {
    use super::super::super::{ApiError, ApiResult};
    use super::super::protocol::{CatalogRecord, Endpoint};

    fn demo_service1() -> CatalogRecord {
        CatalogRecord {
            id: String::from("1"),
            service_type: String::from("identity"),
            name: String::from("keystone"),
            endpoints: vec![
                Endpoint {
                    id: String::from("e1"),
                    interface: String::from("public"),
                    region: String::from("RegionOne"),
                    url: String::from("https://host.one/identity")
                },
                Endpoint {
                    id: String::from("e2"),
                    interface: String::from("internal"),
                    region: String::from("RegionOne"),
                    url: String::from("http://192.168.22.1/identity")
                },
                Endpoint {
                    id: String::from("e3"),
                    interface: String::from("public"),
                    region: String::from("RegionTwo"),
                    url: String::from("https://host.two:5000")
                }
            ]
        }
    }

    fn demo_service2() -> CatalogRecord {
        CatalogRecord {
            id: String::from("2"),
            service_type: String::from("baremetal"),
            name: String::from("ironic"),
            endpoints: vec![
                Endpoint {
                    id: String::from("e4"),
                    interface: String::from("public"),
                    region: String::from("RegionOne"),
                    url: String::from("https://host.one/baremetal")
                },
                Endpoint {
                    id: String::from("e5"),
                    interface: String::from("public"),
                    region: String::from("RegionTwo"),
                    url: String::from("https://host.two:6385")
                }
            ]
        }
    }

    pub fn demo_catalog() -> Vec<CatalogRecord> {
        vec![demo_service1(), demo_service2()]
    }

    fn find_endpoint<'a>(cat: &'a Vec<CatalogRecord>,
                         service_type: &str, interface_type: &str,
                         region: Option<&str>) -> ApiResult<&'a Endpoint> {
        super::find_endpoint(cat, String::from(service_type),
                             String::from(interface_type),
                             region.map(String::from))
    }

    #[test]
    fn test_find_endpoint() {
        let cat = demo_catalog();

        let e1 = find_endpoint(&cat, "identity", "public", None).unwrap();
        assert_eq!(&e1.id, "e1");
        assert_eq!(&e1.url, "https://host.one/identity");

        let e2 = find_endpoint(&cat, "identity", "internal", None).unwrap();
        assert_eq!(&e2.id, "e2");
        assert_eq!(&e2.url, "http://192.168.22.1/identity");

        let e3 = find_endpoint(&cat, "baremetal", "public", None).unwrap();
        assert_eq!(&e3.id, "e4");
        assert_eq!(&e3.url, "https://host.one/baremetal");
    }

    #[test]
    fn test_find_endpoint_with_region() {
        let cat = demo_catalog();

        let e1 = find_endpoint(&cat, "identity", "public",
                               Some("RegionTwo")).unwrap();
        assert_eq!(&e1.id, "e3");
        assert_eq!(&e1.url, "https://host.two:5000");

        let e2 = find_endpoint(&cat, "identity", "internal",
                               Some("RegionOne")).unwrap();
        assert_eq!(&e2.id, "e2");
        assert_eq!(&e2.url, "http://192.168.22.1/identity");

        let e3 = find_endpoint(&cat, "baremetal", "public",
                               Some("RegionTwo")).unwrap();
        assert_eq!(&e3.id, "e5");
        assert_eq!(&e3.url, "https://host.two:6385");
    }

    fn assert_not_found(result: ApiResult<&Endpoint>) {
        match result.err().unwrap() {
            ApiError::EndpointNotFound(..) => (),
            other => panic!("Unexpected error {}", other)
        }
    }

    #[test]
    fn test_find_endpoint_not_found() {
        let cat = demo_catalog();

        assert_not_found(find_endpoint(&cat, "foobar", "public", None));
        assert_not_found(find_endpoint(&cat, "identity", "public",
                                       Some("RegionFoo")));
        assert_not_found(find_endpoint(&cat, "baremetal", "internal", None));
        assert_not_found(find_endpoint(&cat, "identity", "internal",
                                       Some("RegionTwo")));
    }
}
