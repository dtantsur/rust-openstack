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

//! Foundation bits exposing the Compute API.

use reqwest::{Method, Response, StatusCode, Url};
use reqwest::header::Headers;
use serde_json;

use super::super::super::{ApiResult, ApiVersion, Session};
use super::super::super::ApiError::{HttpError, EndpointNotFound};
use super::super::super::service::{ApiVersioning, ServiceInfo, ServiceType,
                                   ServiceWrapper};
use super::super::super::utils;
use super::protocol::{VersionRoot, VersionsRoot};


/// Service wrapper for Compute API V2.
pub type V2ServiceWrapper<'session> = ServiceWrapper<'session, V2>;

/// Service type of Compute API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "compute";
const VERSION_ID: &'static str = "v2.1";

fn extract_info(mut resp: Response, secure: bool) -> ApiResult<ServiceInfo> {
    let body = resp.text()?;

    // First, assume it's a versioned URL.
    let mut info = match serde_json::from_str::<VersionRoot>(&body) {
        Ok(ver) => ver.version.to_service_info(),
        Err(..) => {
            // Second, assume it's a root URL.
            let vers: VersionsRoot = resp.json()?;
            match vers.versions.into_iter().find(|x| &x.id == VERSION_ID) {
                Some(ver) => ver.to_service_info(),
                None => Err(EndpointNotFound(String::from(SERVICE_TYPE)))
            }
        }
    }?;

    // Nova returns insecure URLs even for secure protocol. WHY??
    if secure {
        let _ = info.root_url.set_scheme("https").unwrap();
    }

    Ok(info)
}

impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, session: &Session)
            -> ApiResult<ServiceInfo> {
        debug!("Fetching compute service info from {}", endpoint);
        let secure = endpoint.scheme() == "https";
        let result = session.request(Method::Get, endpoint.clone())?.send()
            .map_err(From::from);
        match result {
            Ok(resp) => {
                let result = extract_info(resp, secure)?;
                info!("Received {:?} from {}", result, endpoint);
                Ok(result)
            },
            Err(HttpError(StatusCode::NotFound, ..)) => {
                if utils::url::is_root(&endpoint) {
                    Err(EndpointNotFound(String::from(SERVICE_TYPE)))
                } else {
                    debug!("Got HTTP 404 from {}, trying parent endpoint",
                           endpoint);
                    V2::service_info(
                        utils::url::pop(endpoint, true),
                        session)
                }
            },
            Err(other) => Err(other)
        }
    }
}

impl ApiVersioning for V2 {
    fn api_version_headers(version: ApiVersion) -> ApiResult<Headers> {
        let mut hdrs = Headers::new();
        // TODO: typed header, new-style header support
        hdrs.set_raw("x-openstack-nova-api-version", version.to_string());
        Ok(hdrs)
    }
}


#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]

    use hyper;
    use hyper::Url;

    use super::super::super::super::{ApiVersion, Session};
    use super::super::super::super::auth::NoAuth;
    use super::super::super::super::service::ServiceType;
    use super::super::super::super::session::test;
    use super::V2;

    // Copied from compute API reference.
    pub const ONE_VERSION_RESPONSE: &'static str = r#"
    {
        "version": {
            "id": "v2.1",
            "links": [
                {
                    "href": "http://openstack.example.com/v2.1/",
                    "rel": "self"
                },
                {
                    "href": "http://docs.openstack.org/",
                    "rel": "describedby",
                    "type": "text/html"
                }
            ],
            "media-types": [
                {
                    "base": "application/json",
                    "type": "application/vnd.openstack.compute+json;version=2.1"
                }
            ],
            "status": "CURRENT",
            "version": "2.42",
            "min_version": "2.1",
            "updated": "2013-07-23T11:33:21Z"
        }
    }"#;

    pub const SEVERAL_VERSIONS_RESPONSE: &'static str = r#"
    {
        "versions": [
            {
                "id": "v2.0",
                "links": [
                    {
                        "href": "http://openstack.example.com/v2/",
                        "rel": "self"
                    }
                ],
                "status": "SUPPORTED",
                "version": "",
                "min_version": "",
                "updated": "2011-01-21T11:33:21Z"
            },
            {
                "id": "v2.1",
                "links": [
                    {
                        "href": "http://openstack.example.com/v2.1/",
                        "rel": "self"
                    }
                ],
                "status": "CURRENT",
                "version": "2.42",
                "min_version": "2.1",
                "updated": "2013-07-23T11:33:21Z"
            }
        ]
    }"#;

    mock_connector_in_order!(MockOneVersion {
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + ONE_VERSION_RESPONSE
    });

    mock_connector_in_order!(MockSeveralVersions {
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + SEVERAL_VERSIONS_RESPONSE
    });

    mock_connector_in_order!(MockOneVersionWithTenant {
        String::from("HTTP/1.1 404 NOT FOUND\r\nServer: Mock.Mock\r\n\r\n{}")
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + ONE_VERSION_RESPONSE
    });

    mock_connector_in_order!(MockSeveralVersionsWithTenant {
        String::from("HTTP/1.1 404 NOT FOUND\r\nServer: Mock.Mock\r\n\r\n{}")
        String::from("HTTP/1.1 200 OK\r\nServer: Mock.Mock\r\n\
                     \r\n") + SEVERAL_VERSIONS_RESPONSE
    });

    mock_connector_in_order!(MockNotFound {
        String::from("HTTP/1.1 404 NOT FOUND\r\nServer: Mock.Mock\r\n\r\n{}")
        String::from("HTTP/1.1 404 NOT FOUND\r\nServer: Mock.Mock\r\n\r\n{}")
    });

    fn prepare_session(cli: hyper::Client) -> Session {
        let auth = NoAuth::new("http://127.0.2.1/v2.1").unwrap();
        test::new_with_params(auth, cli, None)
    }

    fn check_success(cli: hyper::Client, endpoint: &str) {
        let session = prepare_session(cli);
        let url = Url::parse(endpoint).unwrap();
        let info = V2::service_info(url, &session).unwrap();
        assert_eq!(info.root_url.as_str(),
                   "http://openstack.example.com/v2.1/");
        assert_eq!(info.current_version.unwrap(), ApiVersion(2, 42));
        assert_eq!(info.minimum_version.unwrap(), ApiVersion(2, 1));
    }

    #[test]
    fn test_one_version() {
        let cli = hyper::Client::with_connector(MockOneVersion::default());
        check_success(cli, "http://127.0.2.1/compute/v2.1");
    }

    #[test]
    fn test_one_version_with_tenant() {
        let cli = hyper::Client::with_connector(
            MockOneVersionWithTenant::default());
        check_success(cli, "http://127.0.2.1/compute/v2.1/tenant");
    }

    #[test]
    fn test_several_version() {
        let cli = hyper::Client::with_connector(
            MockSeveralVersions::default());
        check_success(cli, "http://127.0.2.1/");
    }

    #[test]
    fn test_several_version_with_tenant() {
        let cli = hyper::Client::with_connector(
            MockSeveralVersionsWithTenant::default());
        check_success(cli, "http://127.0.2.1/tenant");
    }
}
