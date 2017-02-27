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

//! OpenStack Identity V3 API support for access tokens.

use std::env;
use std::io::Read;

use hyper::{Client, Url};
use hyper::Error as HttpClientError;
use hyper::client::IntoUrl;
use hyper::error::ParseError;
use hyper::header::ContentType;
use hyper::status::StatusCode;
use rustc_serialize::json;

use super::base::{AuthError, AuthMethod, AuthToken, SubjectTokenHeader};


#[derive(Clone, RustcDecodable, RustcEncodable)]
struct Domain {
    name: String
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
struct User {
    name: String,
    password: String,
    domain: Domain
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
struct PasswordAuth {
    user: User
}


#[derive(Clone, RustcDecodable, RustcEncodable)]
struct PasswordIdentity {
    methods: Vec<String>,
    password: PasswordAuth
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
struct Project {
    name: String,
    domain: Domain
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
struct ProjectScope {
    project: Project
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
struct ScopedAuth {
    identity: PasswordIdentity,
    scope: ProjectScope
}

const PASSWORD_METHOD: &'static str = "password";
const MISSING_USER: &'static str = "User information required";
const MISSING_SCOPE: &'static str = "Unscoped tokens are not supported now";
const MISSING_ENV_VARS: &'static str =
    "Not all required environment variables were provided";


/// Authentication method factory using Identity API V3.
#[derive(Clone)]
pub struct Identity {
    auth_url: Url,
    password_auth: Option<PasswordAuth>,
    project_scope: Option<ProjectScope>
}

/// Authentication method using Identity API V3.
///
/// Should be created via Identity struct methods.
#[derive(Clone)]
pub struct IdentityAuthMethod {
    auth_url: Url,
    body: ScopedAuth
}

impl Identity{
    /// Create a password authentication against the given Identity service.
    pub fn new<U>(auth_url: U) -> Result<Identity, ParseError> where U: IntoUrl  {
        let real_url = try!(auth_url.into_url());
        Ok(Identity {
            auth_url: real_url,
            password_auth: None,
            project_scope: None
        })
    }

    /// Add authentication based on user name and password.
    pub fn with_user<S1, S2, S3>(self, user_name: S1, password: S2,
                                 domain_name: S3) -> Identity
            where S1: Into<String>, S2: Into<String>, S3: Into<String> {
        Identity {
            password_auth: Some(PasswordAuth {
                user: User {
                    name: user_name.into(),
                    password: password.into(),
                    domain: Domain {
                        name: domain_name.into()
                    }
                }
            }),
            .. self
        }
    }

    /// Request a token scoped to the given project.
    pub fn with_project_scope<S1, S2>(self, project_name: S1, domain_name: S2)
            -> Identity where S1: Into<String>, S2: Into<String> {
        Identity {
            project_scope: Some(ProjectScope {
                project: Project {
                    name: project_name.into(),
                    domain: Domain {
                        name: domain_name.into()
                    }
                }
            }),
            .. self
        }
    }

    /// Create an authentication method based on provided information.
    pub fn create(self) -> Result<IdentityAuthMethod, AuthError> {
        /// TODO: support more authentication methods (at least a token)
        let password_auth = match self.password_auth {
            Some(p) => p,
            None =>
                return Err(AuthError::InsufficientCredentials(MISSING_USER))
        };

        /// TODO: support unscoped tokens
        let project_scope = match self.project_scope {
            Some(p) => p,
            None =>
                return Err(AuthError::InsufficientCredentials(MISSING_SCOPE))
        };

        Ok(IdentityAuthMethod {
            auth_url: self.auth_url,
            body: ScopedAuth {
                identity: PasswordIdentity {
                    methods: vec![String::from(PASSWORD_METHOD)],
                    password: password_auth
                },
                scope: project_scope
            }
        })
    }

    /// Create an authentication method from environment variables.
    pub fn from_env() -> Result<IdentityAuthMethod, AuthError> {
        let auth_url = try!(_get_env("OS_AUTH_URL"));
        let id = match Identity::new(&auth_url) {
            Ok(x) => x,
            Err(e) =>
                return Err(AuthError::ProtocolError(HttpClientError::Uri(e)))
        };

        let user_name = try!(_get_env("OS_USERNAME"));
        let password = try!(_get_env("OS_PASSWORD"));
        let project_name = try!(_get_env("OS_PROJECT_NAME"));

        let user_domain = env::var("OS_USER_DOMAIN_NAME")
            .unwrap_or(String::from("Default"));
        let project_domain = env::var("OS_PROJECT_DOMAIN_NAME")
            .unwrap_or(String::from("Default"));

        id.with_user(user_name, password, user_domain)
            .with_project_scope(project_name, project_domain)
            .create()
    }
}

fn _get_env(name: &str) -> Result<String, AuthError> {
    env::var(name).or(
        Err(AuthError::InsufficientCredentials(MISSING_ENV_VARS)))
}

impl AuthMethod for IdentityAuthMethod {
    /// Verify authentication and generate an auth token.
    fn get_token(&mut self, client: &Client) -> Result<AuthToken, AuthError> {
        // TODO: allow /v3 postfix built into auth_url?
        let url = format!("{}/v3/auth/tokens", self.auth_url.to_string());
        debug!("Requesting a token for user {} from {}",
               self.body.identity.password.user.name, url);
        let body = json::encode(&self.body).unwrap();
        let json_type = ContentType(mime!(Application/Json));

        let mut resp = try!(client.post(&url).body(&body)
                            .header(json_type).send());

        let mut resp_body = String::new();
        try!(resp.read_to_string(&mut resp_body));

        let token_value = match resp.status {
            StatusCode::Ok | StatusCode::Created => {
                let header: Option<&SubjectTokenHeader> = resp.headers.get();
                match header {
                    Some(ref value) => value.0.clone(),
                    None => return Err(AuthError::ProtocolError(
                            HttpClientError::Header))
                }
            },
            StatusCode::Unauthorized => {
                warn!("Invalid credentials for user {}",
                      self.body.identity.password.user.name);
                return Err(AuthError::Unauthorized);
            },
            other => {
                error!("Unexpected HTTP error {} when getting a token for {}",
                       other, self.body.identity.password.user.name);
                return Err(AuthError::HttpError(other, Some(resp_body)));
            }
        };

        debug!("Received a token for user {} from {}",
               self.body.identity.password.user.name, url);

        // TODO: detect expiration time
        // TODO: do something useful about the body
        Ok(AuthToken {
            token: token_value,
            expires_at: None
        })
    }

    /// Get a URL for the request service (NOT IMPLEMENTED).
    fn get_endpoint(&mut self, _service_type: &str, _client: &Client)
            -> Result<Url, AuthError> {
        // TODO: implement
        Err(AuthError::EndpointNotFound)
    }
}

#[cfg(test)]
pub mod test {
    use super::Identity;

    #[test]
    fn test_identity_new() {
        let id = Identity::new("http://127.0.0.1:8080/").unwrap();
        let e = id.auth_url;
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/");
    }

    #[test]
    fn test_identity_new_invalid() {
        Identity::new("http://127.0.0.1 8080/").err().unwrap();
    }
}
