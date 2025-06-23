// Copyright 2021 Dmitry Tantsur <dtantsur@protonmail.com>
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

//! Abstraction over a service catalog.

use std::fmt;

use log::{debug, error};
use osauth::{Error, ErrorKind};
use reqwest::Url;

use super::protocol;
use crate::EndpointFilters;

/// Abstraction over a service catalog.
///
/// In standalone case only one URL is returned for any service.
#[derive(Debug, Clone)]
pub struct ServiceCatalog {
    inner: Vec<protocol::CatalogRecord>,
}

fn new_endpoint_not_found<D: fmt::Display>(service_type: D) -> Error {
    Error::new(
        ErrorKind::EndpointNotFound,
        format!("Endpoint for service {} was not found", service_type),
    )
}

impl ServiceCatalog {
    pub(crate) fn new(catalog: Vec<protocol::CatalogRecord>) -> ServiceCatalog {
        ServiceCatalog { inner: catalog }
    }

    /// Find an endpoint in the catalog.
    pub fn find_endpoint(
        &self,
        service_type: &str,
        filters: &EndpointFilters,
    ) -> Result<Url, Error> {
        let svc = match self.inner.iter().find(|x| x.service_type == *service_type) {
            Some(s) => s,
            None => return Err(new_endpoint_not_found(service_type)),
        };

        let mut endpoints: Vec<_> = svc
            .endpoints
            .iter()
            .filter(|x| {
                filters
                    .interfaces
                    // FIXME(dtantsur): return to using check() when migrated
                    .iter()
                    .position(|item| item == &x.interface)
                    .is_some()
            })
            .collect();
        endpoints
            // NOTE(dtantsur): because of the filter above unwrap never fails
            .sort_unstable_by_key(|x| {
                filters
                    .interfaces
                    // FIXME(dtantsur): return to using find() when migrated
                    .iter()
                    .position(|item| item == &x.interface)
                    .unwrap()
            });
        endpoints
            .into_iter()
            .next()
            .ok_or_else(|| new_endpoint_not_found(service_type))
            .and_then(|endp| {
                debug!("Received {:?} for {}", endp, service_type);
                Url::parse(&endp.url).map_err(|e| {
                    error!(
                        "Invalid URL {} received from service catalog for service \
                     '{}', filters {:?}: {}",
                        endp.url, service_type, filters, e
                    );
                    Error::new(
                        ErrorKind::InvalidResponse,
                        format!("Invalid URL {} for {} - {}", endp.url, service_type, e),
                    )
                })
            })
    }
}

#[cfg(test)]
pub mod test {
    use reqwest::Url;

    use crate::identity::protocol::{CatalogRecord, Endpoint};
    use crate::{EndpointFilters, Error, ErrorKind, InterfaceType, ValidInterfaces};
    use InterfaceType::*;

    use super::ServiceCatalog;

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

    pub fn demo_catalog() -> ServiceCatalog {
        ServiceCatalog::new(vec![demo_service1(), demo_service2()])
    }

    fn find_endpoint(
        cat: &ServiceCatalog,
        service_type: &str,
        interface_type: InterfaceType,
        region: Option<&str>,
    ) -> Result<Url, Error> {
        let filters = EndpointFilters {
            interfaces: ValidInterfaces::one(interface_type),
            region: region.map(|x| x.to_string()),
        };
        cat.find_endpoint(service_type, &filters)
    }

    #[test]
    fn test_find_endpoint() {
        let cat = demo_catalog();

        let e1 = find_endpoint(&cat, "identity", Public, None).unwrap();
        assert_eq!(e1.as_str(), "https://host.one/identity");

        let e2 = find_endpoint(&cat, "identity", Internal, None).unwrap();
        assert_eq!(e2.as_str(), "http://192.168.22.1/identity");

        let e3 = find_endpoint(&cat, "baremetal", Public, None).unwrap();
        assert_eq!(e3.as_str(), "https://host.one/baremetal");
    }

    #[test]
    fn test_find_endpoint_from_many() {
        let cat = demo_catalog();
        let service_type = "identity";

        let f1 = EndpointFilters::default().with_interfaces(vec![Public, Internal]);
        let e1 = cat.find_endpoint(service_type, &f1).unwrap();
        assert_eq!(e1.as_str(), "https://host.one/identity");

        let f2 = EndpointFilters::default().with_interfaces(vec![Admin, Internal, Public]);
        let e2 = cat.find_endpoint(service_type, &f2).unwrap();
        assert_eq!(e2.as_str(), "http://192.168.22.1/identity");

        let f3 = EndpointFilters::default().with_interfaces(vec![Admin, Public]);
        let e3 = cat.find_endpoint(service_type, &f3).unwrap();
        assert_eq!(e3.as_str(), "https://host.one/identity");
    }

    #[test]
    fn test_find_endpoint_with_region() {
        let cat = demo_catalog();

        let e1 = find_endpoint(&cat, "identity", Public, Some("RegionTwo")).unwrap();
        assert_eq!(e1.as_str(), "https://host.two:5000/");

        let e2 = find_endpoint(&cat, "identity", Internal, Some("RegionOne")).unwrap();
        assert_eq!(e2.as_str(), "http://192.168.22.1/identity");

        let e3 = find_endpoint(&cat, "baremetal", Public, Some("RegionTwo")).unwrap();
        assert_eq!(e3.as_str(), "https://host.two:6385/");
    }

    fn assert_not_found(result: Result<Url, Error>) {
        let err = result.err().unwrap();
        if err.kind() != ErrorKind::EndpointNotFound {
            panic!("Unexpected error {}", err);
        }
    }

    #[test]
    fn test_find_endpoint_not_found() {
        let cat = demo_catalog();

        assert_not_found(find_endpoint(&cat, "foobar", Public, None));
        assert_not_found(find_endpoint(&cat, "identity", Public, Some("RegionFoo")));
        assert_not_found(find_endpoint(&cat, "baremetal", Internal, None));
        assert_not_found(find_endpoint(&cat, "identity", Internal, Some("RegionTwo")));

        let f1 = EndpointFilters::default().with_interfaces(vec![Admin, Internal]);
        let e1 = cat.find_endpoint("baremetal", &f1);
        assert_not_found(e1);
    }
}
