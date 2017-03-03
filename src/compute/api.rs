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

//! Low-level Compute API implementation.

use hyper;

use super::super::auth::AuthMethod;
use super::super::{ApiError, Session};
use super::protocol;

/// List of servers.
pub type ServerList = Vec<protocol::Server>;

/// Low-level Compute API calls.
#[derive(Debug)]
pub struct ComputeApi<'a, A: AuthMethod + 'a> {
    session: &'a Session<A>,
    endpoint_interface: Option<String>,
    region: Option<String>
}

const SERVICE_TYPE: &'static str = "compute";

impl<'a, A: AuthMethod + 'a> ComputeApi<'a, A> {
    /// Create a new API instance using the given session.
    pub fn new(session: &'a Session<A>) -> ComputeApi<'a, A> {
        ComputeApi::new_with_endpoint_params(session, None, None)
    }

    /// Create a new API instance using the given session.
    ///
    /// This variant allows passing an endpoint type (defaults to public),
    /// and region (defaults to any).
    pub fn new_with_endpoint_params(session: &'a Session<A>,
                                    endpoint_interface: Option<&str>,
                                    region: Option<&str>)
            -> ComputeApi<'a, A> {
        ComputeApi {
            session: session,
            endpoint_interface: endpoint_interface.map(String::from),
            region: region.map(String::from)
        }
    }

    fn get_endpoint(&self, path: &str) -> Result<hyper::Url, ApiError> {
        // TODO: move this code to Session
        let endpoint = try!(self.session.get_endpoint(
                SERVICE_TYPE,
                self.endpoint_interface.as_ref().map(String::as_str),
                self.region.as_ref().map(String::as_str)));

        let with_version = if endpoint.path().ends_with("/v2.1") {
            endpoint
        } else {
            try!(endpoint.join("v2.1"))
        };

        with_version.join(path).map_err(From::from)
    }

    /// List servers.
    pub fn list_servers(&self) -> Result<ServerList, ApiError> {
        let url = try!(self.get_endpoint("servers"));
        debug!("Listing servers from {}", url);
        let resp = try!(self.session.request(hyper::Get, url).send());
        let root = try!(protocol::ServersRoot::from_reader(resp));
        Ok(root.servers)
    }
}

#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use hyper;

    use super::super::super::Session;
    use super::super::super::auth::base::{NoAuth, SimpleAuthToken};
    use super::ComputeApi;

    // Copied from compute API reference.
    const SERVERS_RESPONSE: &'static str = r#"
    {
        "servers": [
            {
                "id": "22c91117-08de-4894-9aa9-6ef382400985",
                "links": [
                    {
                        "href": "http://openstack.example.com/v2/6f70656e737461636b20342065766572/servers/22c91117-08de-4894-9aa9-6ef382400985",
                        "rel": "self"
                    },
                    {
                        "href": "http://openstack.example.com/6f70656e737461636b20342065766572/servers/22c91117-08de-4894-9aa9-6ef382400985",
                        "rel": "bookmark"
                    }
                ],
                "name": "new-server-test"
            }
        ]
    }"#;

    mock_connector!(MockServers {
        "http://127.0.2.1" => String::from("HTTP/1.1 200 OK\r\n\
                                           Server: Mock.Mock\r\n\
                                           \r\n") + SERVERS_RESPONSE
    });

    #[test]
    fn test_servers_list() {
        let auth = NoAuth::new("http://127.0.2.1/v2.1").unwrap();
        let cli = hyper::Client::with_connector(MockServers::default());
        let token = SimpleAuthToken(String::from("abcdef"));
        let session = Session::new_with_params(auth, cli, token);

        let api = ComputeApi::new(&session);
        let srvs = api.list_servers().unwrap();
        assert_eq!(srvs.len(), 1);
        assert_eq!(&srvs[0].id, "22c91117-08de-4894-9aa9-6ef382400985");
        assert_eq!(&srvs[0].name, "new-server-test");
    }
}
