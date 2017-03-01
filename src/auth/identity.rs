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
use hyper::client::{IntoUrl, Response};
use hyper::error::ParseError;
use hyper::header::ContentType;
use hyper::status::StatusCode;

use super::super::ApiError;
use super::super::identity::catalog;
use super::super::identity::protocol;
use super::super::session::AuthenticatedClient;
use super::base::{AuthMethod, AuthToken, SubjectTokenHeader};


const MISSING_USER: &'static str = "User information required";
const MISSING_SCOPE: &'static str = "Unscoped tokens are not supported now";
const MISSING_ENV_VARS: &'static str =
    "Not all required environment variables were provided";


/// Authentication method factory using Identity API V3.
#[derive(Clone)]
pub struct Identity {
    auth_url: Url,
    password_identity: Option<protocol::PasswordIdentity>,
    project_scope: Option<protocol::ProjectScope>
}

/// Authentication method using Identity API V3.
///
/// Should be created via Identity struct methods.
#[derive(Clone)]
pub struct IdentityAuthMethod {
    auth_url: Url,
    body: protocol::ProjectScopedAuthRoot,
    token_endpoint: String
}

impl Identity {
    /// Get a reference to the auth URL.
    pub fn get_auth_url(&self) -> &Url {
        &self.auth_url
    }

    /// Create a password authentication against the given Identity service.
    pub fn new<U>(auth_url: U) -> Result<Identity, ParseError> where U: IntoUrl  {
        let real_url = try!(auth_url.into_url());
        Ok(Identity {
            auth_url: real_url,
            password_identity: None,
            project_scope: None
        })
    }

    /// Add authentication based on user name and password.
    pub fn with_user<S1, S2, S3>(self, user_name: S1, password: S2,
                                 domain_name: S3) -> Identity
            where S1: Into<String>, S2: Into<String>, S3: Into<String> {
        Identity {
            password_identity: Some(protocol::PasswordIdentity::new(user_name,
                                                                    password,
                                                                    domain_name)),
            .. self
        }
    }

    /// Request a token scoped to the given project.
    pub fn with_project_scope<S1, S2>(self, project_name: S1, domain_name: S2)
            -> Identity where S1: Into<String>, S2: Into<String> {
        Identity {
            project_scope: Some(protocol::ProjectScope::new(project_name,
                                                            domain_name)),
            .. self
        }
    }

    /// Create an authentication method based on provided information.
    pub fn create(self) -> Result<IdentityAuthMethod, ApiError> {
        /// TODO: support more authentication methods (at least a token)
        let password_identity = match self.password_identity {
            Some(p) => p,
            None =>
                return Err(ApiError::InsufficientCredentials(MISSING_USER))
        };

        /// TODO: support unscoped tokens
        let project_scope = match self.project_scope {
            Some(p) => p,
            None =>
                return Err(ApiError::InsufficientCredentials(MISSING_SCOPE))
        };

        Ok(IdentityAuthMethod::new(self.auth_url, password_identity,
                                   project_scope))
    }

    /// Create an authentication method from environment variables.
    pub fn from_env() -> Result<IdentityAuthMethod, ApiError> {
        let auth_url = try!(_get_env("OS_AUTH_URL"));
        let id = match Identity::new(&auth_url) {
            Ok(x) => x,
            Err(e) =>
                return Err(ApiError::ProtocolError(HttpClientError::Uri(e)))
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

fn _get_env(name: &str) -> Result<String, ApiError> {
    env::var(name).or(
        Err(ApiError::InsufficientCredentials(MISSING_ENV_VARS)))
}

impl IdentityAuthMethod {
    /// Get a reference to the auth URL.
    pub fn get_auth_url(&self) -> &Url {
        &self.auth_url
    }

    fn new(auth_url: Url, password_identity: protocol::PasswordIdentity,
           project_scope: protocol::ProjectScope) -> IdentityAuthMethod {
        let body = protocol::ProjectScopedAuthRoot::new(password_identity,
                                                        project_scope);
        // TODO: allow /v3 postfix built into auth_url?
        let token_endpoint = format!("{}/v3/auth/tokens",
                                     auth_url.to_string());
        IdentityAuthMethod {
            auth_url: auth_url,
            body: body,
            token_endpoint: token_endpoint
        }
    }

    fn token_from_response(&self, resp: &mut Response)
            -> Result<AuthToken, ApiError> {
        let mut resp_body = String::new();
        try!(resp.read_to_string(&mut resp_body));

        let token_value = match resp.status {
            StatusCode::Ok | StatusCode::Created => {
                let header: Option<&SubjectTokenHeader> = resp.headers.get();
                match header {
                    Some(ref value) => value.0.clone(),
                    None => return Err(
                        ApiError::ProtocolError(HttpClientError::Header))
                }
            },
            StatusCode::Unauthorized => {
                warn!("Invalid credentials for user {}",
                      self.body.auth.identity.password.user.name);
                return Err(ApiError::Unauthorized);
            },
            other => {
                error!("Unexpected HTTP error {} when getting a token for {}",
                       other, self.body.auth.identity.password.user.name);
                return Err(ApiError::HttpError(other, Some(resp_body)));
            }
        };

        debug!("Received a token for user {} from {}",
               self.body.auth.identity.password.user.name,
               self.token_endpoint);

        // TODO: detect expiration time
        // TODO: do something useful about the body
        Ok(AuthToken {
            token: token_value,
            expires_at: None
        })
    }
}

impl AuthMethod for IdentityAuthMethod {
    /// Verify authentication and generate an auth token.
    fn get_token(&self, client: &Client) -> Result<AuthToken, ApiError> {
        debug!("Requesting a token for user {} from {}",
               self.body.auth.identity.password.user.name,
               self.token_endpoint);

        let body = self.body.to_string().unwrap();
        let json_type = ContentType(mime!(Application/Json));
        let mut resp = try!(client.post(&self.token_endpoint).body(&body)
                            .header(json_type).send());
        self.token_from_response(&mut resp)
    }

    /// Get a URL for the requested service.
    fn get_endpoint(&self, service_type: &str,
                    endpoint_interface: Option<&str>,
                    region: Option<&str>, client: &AuthenticatedClient)
            -> Result<Url, ApiError> {
        let real_interface = endpoint_interface.unwrap_or("public");
        let cat = try!(catalog::get_service_catalog(&self.auth_url, client));
        let endp = try!(catalog::find_endpoint(&cat, service_type,
                                               real_interface, region));
        endp.url.into_url().map_err(From::from)
    }
}

#[cfg(test)]
pub mod test {
    use hyper;

    use super::super::super::ApiError;
    use super::super::base::AuthMethod;
    use super::Identity;

    mock_connector!(MockToken {
        "http://127.0.1.1" => "HTTP/1.1 200 OK\r\n\
                               Server: Mock.Mock\r\n\
                               X-Subject-Token: abcdef\r\n
                               \r\n\
                               "
        "http://127.0.1.2" => "HTTP/1.1 401 Unauthorized\r\n\
                               Server: Mock.Mock\r\n\
                               \r\n\
                               boom"
        "http://127.0.1.3" => "HTTP/1.1 404 Not Found\r\n\
                               Server: Mock.Mock\r\n\
                               \r\n\
                               nothing found"
    });

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

    #[test]
    fn test_identity_create() {
        let id = Identity::new("http://127.0.0.1:8080/identity").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        assert_eq!(&id.auth_url.to_string(), "http://127.0.0.1:8080/identity");
        assert_eq!(id.get_auth_url().to_string(),
                   "http://127.0.0.1:8080/identity");
        assert_eq!(&id.body.auth.identity.password.user.name, "user");
        assert_eq!(&id.body.auth.identity.password.user.password, "pa$$w0rd");
        assert_eq!(&id.body.auth.identity.password.user.domain.name,
                   "example.com");
        assert_eq!(id.body.auth.identity.methods,
                   vec![String::from("password")]);
        assert_eq!(&id.body.auth.scope.project.name, "cool project");
        assert_eq!(&id.body.auth.scope.project.domain.name, "example.com");
        assert_eq!(&id.token_endpoint,
                   "http://127.0.0.1:8080/identity/v3/auth/tokens");
    }

    #[test]
    fn test_identity_create_no_scope() {
        Identity::new("http://127.0.0.1:8080/identity").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .create().err().unwrap();
    }

    #[test]
    fn test_identity_create_no_user() {
        Identity::new("http://127.0.0.1:8080/identity").unwrap()
            .with_project_scope("cool project", "example.com")
            .create().err().unwrap();
    }

    #[test]
    fn test_identity_get_token() {
        let id = Identity::new("http://127.0.1.1").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockToken::default());
        let token = id.get_token(&cli).unwrap();
        assert_eq!(&token.token, "abcdef");
    }

    #[test]
    fn test_identity_get_token_unauthorized() {
        let id = Identity::new("http://127.0.1.2").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockToken::default());
        match id.get_token(&cli).err().unwrap() {
            ApiError::Unauthorized => (),
            other => panic!("Unexpected {}", other)
        };
    }

    #[test]
    fn test_identity_get_token_fail() {
        let id = Identity::new("http://127.0.1.3").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockToken::default());
        match id.get_token(&cli).err().unwrap() {
            ApiError::HttpError(code, ref s) => {
                assert_eq!(code, hyper::NotFound);
                assert_eq!(s.clone().unwrap(), "nothing found");
            },
            other => panic!("Unexpected {}", other)
        };
    }
}
