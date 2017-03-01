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

//! Session structure definition.
//!
//! The Session object serves as a wrapper around an HTTP(s) client, handling
//! authentication, accessing the service catalog and token refresh.

use hyper::Client;
use hyper::client::{IntoUrl, RequestBuilder};
use hyper::method::Method;

use super::auth::base::{AuthToken, AuthTokenHeader};


/// An HTTP(s) client with authentication built-in.
#[derive(Debug)]
pub struct AuthenticatedClient {
    client: Client,
    token: AuthToken
}

impl AuthenticatedClient {
    /// Create an authenticated client from an HTTP client and a token.
    pub fn new(client: Client, token: AuthToken) -> AuthenticatedClient {
        AuthenticatedClient {
            client: client,
            token: token
        }
    }

    /// Get a reference to the authentication token.
    pub fn auth_token(&self) -> &AuthToken {
        &self.token
    }

    /// Get a reference to the underlying client object.
    pub fn raw_client(&self) -> &Client {
        &self.client
    }

    /// A wrapper for HTTP request.
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        let req = self.client.request(method, url);
        req.header(AuthTokenHeader(self.token.token.clone()))
    }
}

#[cfg(test)]
pub mod test {
    use super::super::auth::base::AuthToken;
    use super::super::utils;

    use super::AuthenticatedClient;

    pub fn new_client(token: &str) -> AuthenticatedClient {
        let token = AuthToken {
            token: String::from(token),
            expires_at: None
        };

        AuthenticatedClient::new(utils::http_client(), token)
    }


    #[test]
    fn test_authenticatedclient_new() {
        let cli = new_client("foo");
        assert_eq!(&cli.token.token, "foo");
    }
}
