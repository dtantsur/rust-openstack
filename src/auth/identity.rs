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

use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fmt;
use std::hash::{Hash, Hasher};

use reqwest::{Client, IntoUrl, Method, RequestBuilder, Response, StatusCode,
              Url, UrlError};
use reqwest::header::{ContentType, Headers};

use super::super::{ApiError, ApiResult};
use super::super::identity::{catalog, protocol};
use super::super::utils::ValueCache;
use super::AuthMethod;

use ApiError::InvalidInput;


const MISSING_USER: &'static str = "User information required";
const MISSING_SCOPE: &'static str = "Unscoped tokens are not supported now";
const MISSING_ENV_VARS: &'static str =
    "Not all required environment variables were provided";
const INVALID_ENV_AUTH_URL: &'static str =
    "Malformed authentication URL provided in the environment";
const MISSING_SUBJECT_HEADER: &'static str =
    "Missing X-Subject-Token header";
const INVALID_URL: &'static str =
    "Invalid URL received from service catalog";


/// Plain authentication token without additional details.
#[derive(Clone)]
struct Token(pub String);

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        write!(f, "Token {{ hash: {} }}", hasher.finish())
    }
}

impl Hash for Token {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.0.hash(state);
    }
}


/// Authentication method factory using Identity API V3.
#[derive(Clone, Debug)]
pub struct Identity {
    auth_url: Url,
    password_identity: Option<protocol::PasswordIdentity>,
    project_scope: Option<protocol::ProjectScope>
}

/// Password authentication using Identity API V3.
///
/// Has to be created via [Identity object](struct.Identity.html) methods.
#[derive(Clone, Debug)]
pub struct PasswordAuth {
    auth_url: Url,
    body: protocol::ProjectScopedAuthRoot,
    token_endpoint: String,
    cached_token: ValueCache<Token>
}

impl Identity {
    /// Get a reference to the auth URL.
    pub fn get_auth_url(&self) -> &Url {
        &self.auth_url
    }

    /// Create a password authentication against the given Identity service.
    pub fn new<U>(auth_url: U) -> Result<Identity, UrlError> where U: IntoUrl  {
        Ok(Identity {
            auth_url: auth_url.into_url()?,
            password_identity: None,
            project_scope: None,
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
    pub fn create(self) -> ApiResult<PasswordAuth> {
        // TODO: support more authentication methods (at least a token)
        let password_identity = match self.password_identity {
            Some(p) => p,
            None =>
                return Err(
                    InvalidInput(String::from(MISSING_USER))
                )
        };

        // TODO: support unscoped tokens
        let project_scope = match self.project_scope {
            Some(p) => p,
            None =>
                return Err(
                    InvalidInput(String::from(MISSING_SCOPE))
                )
        };

        Ok(PasswordAuth::new(self.auth_url, password_identity, project_scope))
    }

    /// Create an authentication method from environment variables.
    pub fn from_env() -> ApiResult<PasswordAuth> {
        let auth_url = _get_env("OS_AUTH_URL")?;
        let id = Identity::new(&auth_url).map_err(|_| {
            InvalidInput(String::from(INVALID_ENV_AUTH_URL))
        })?;

        let user_name = _get_env("OS_USERNAME")?;
        let password = _get_env("OS_PASSWORD")?;
        let project_name = _get_env("OS_PROJECT_NAME")?;

        let user_domain = env::var("OS_USER_DOMAIN_NAME")
            .unwrap_or(String::from("Default"));
        let project_domain = env::var("OS_PROJECT_DOMAIN_NAME")
            .unwrap_or(String::from("Default"));

        id.with_user(user_name, password, user_domain)
            .with_project_scope(project_name, project_domain)
            .create()
    }
}

#[inline]
fn _get_env(name: &str) -> ApiResult<String> {
    env::var(name).or(Err(InvalidInput(String::from(MISSING_ENV_VARS))))
}

#[inline]
fn extract_subject_token(headers: &Headers) -> Option<String> {
    // TODO: replace with a typed header
    headers.get_raw("x-subject-token").and_then(|h| h.one())
        .map(|buf| { String::from_utf8_lossy(buf).into_owned() })
}

impl PasswordAuth {
    /// Get a reference to the auth URL.
    pub fn get_auth_url(&self) -> &Url {
        &self.auth_url
    }

    fn new(auth_url: Url, password_identity: protocol::PasswordIdentity,
           project_scope: protocol::ProjectScope) -> PasswordAuth {
        let body = protocol::ProjectScopedAuthRoot::new(password_identity,
                                                        project_scope);
        // TODO: more robust logic?
        let token_endpoint = if auth_url.path().ends_with("/v3") {
            format!("{}/auth/tokens", auth_url)
        } else {
            format!("{}/v3/auth/tokens", auth_url)
        };

        PasswordAuth {
            auth_url: auth_url,
            body: body,
            token_endpoint: token_endpoint,
            cached_token: ValueCache::new(None)
        }
    }

    fn token_from_response(&self, resp: Response) -> ApiResult<Token> {
        let token_value = match resp.status() {
            StatusCode::Ok | StatusCode::Created => {
                match extract_subject_token(resp.headers()) {
                    Some(value) => value,
                    None => {
                        error!("No X-Subject-Token header received from {}",
                               self.token_endpoint);
                        return Err(
                            ApiError::InvalidResponse(
                                String::from(MISSING_SUBJECT_HEADER)))
                    }
                }
            },
            StatusCode::Unauthorized => {
                error!("Invalid credentials for user {}",
                       self.body.auth.identity.password.user.name);
                return Err(ApiError::HttpError(resp.status(), resp));
            },
            other => {
                error!("Unexpected HTTP error {} when getting a token for {}",
                       other, self.body.auth.identity.password.user.name);
                return Err(ApiError::HttpError(resp.status(), resp));
            }
        };

        info!("Received a token for user {} from {}",
               self.body.auth.identity.password.user.name,
               self.token_endpoint);

        // TODO: detect expiration time
        // TODO: do something useful about the body
        Ok(Token(token_value))
    }

    fn refresh_token(&self, client: &Client) -> ApiResult<()> {
        // TODO: refresh on expiration
        self.cached_token.ensure_value(|| {
            debug!("Requesting a token for user {} from {}",
                   self.body.auth.identity.password.user.name,
                   self.token_endpoint);
            let resp = client.post(&self.token_endpoint).json(&self.body)
                .header(ContentType::json()).send()?;
            self.token_from_response(resp)
        })
    }

    fn get_token(&self, client: &Client) -> ApiResult<String> {
        self.refresh_token(client)?;
        Ok(self.cached_token.get().unwrap().0)
    }

    fn get_catalog(&self, client: &Client)
            -> ApiResult<Vec<protocol::CatalogRecord>> {
        // TODO: catalog caching
        let catalog_url = catalog::get_url(self.auth_url.clone());
        trace!("Requesting a service catalog from {}", catalog_url);
        let mut req = self.request(client, Method::Get, catalog_url)?;
        let body: protocol::CatalogRoot = req.send()?.json()?;
        trace!("Received catalog: {:?}", body.catalog);
        Ok(body.catalog)
    }
}

impl AuthMethod for PasswordAuth {
    /// Create an authenticated request.
    fn request(&self, client: &Client, method: Method, url: Url) -> ApiResult<RequestBuilder> {
        let token = self.get_token(client)?;
        let mut headers = Headers::new();
        // TODO: replace with a typed header
        headers.set_raw("x-auth-token", token);
        let mut builder = client.request(method, url);
        {
            let _unused = builder.headers(headers);
        }
        Ok(builder)
    }

    /// Get a URL for the requested service.
    fn get_endpoint(&self, client: &Client,
                    service_type: String,
                    endpoint_interface: Option<String>,
                    region: Option<String>) -> ApiResult<Url> {
        let real_interface = endpoint_interface.unwrap_or(
            self.default_endpoint_interface());
        debug!("Requesting a catalog endpoint for service '{}', interface \
               '{}' from region {:?}", service_type, real_interface, region);
        let cat = self.get_catalog(client)?;
        let endp = catalog::find_endpoint(&cat, service_type,
                                          real_interface, region)?;
        info!("Received {:?}", endp);
        Url::parse(&endp.url).map_err(|e| {
            error!("Invalid URL {} received from service catalog: {}",
                   endp.url, e);
            ApiError::InvalidResponse(String::from(INVALID_URL))
        })
    }
}

#[cfg(test)]
pub mod test {
    #![allow(missing_debug_implementations)]
    #![allow(unused_results)]

    use hyper::{self, Url};
    use hyper::status::StatusCode;

    use super::super::super::{ApiError, ApiResult};
    use super::super::AuthMethod;
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

    // Copied from keystone API reference.
    const EXAMPLE_CATALOG_RESPONSE: &'static str = r#"
    {
        "catalog": [
            {
                "endpoints": [
                    {
                        "id": "39dc322ce86c4111b4f06c2eeae0841b",
                        "interface": "public",
                        "region": "RegionOne",
                        "url": "http://localhost:5000"
                    },
                    {
                        "id": "ec642f27474842e78bf059f6c48f4e99",
                        "interface": "internal",
                        "region": "RegionOne",
                        "url": "http://localhost:5000"
                    },
                    {
                        "id": "c609fc430175452290b62a4242e8a7e8",
                        "interface": "admin",
                        "region": "RegionOne",
                        "url": "http://localhost:35357"
                    }
                ],
                "id": "4363ae44bdf34a3981fde3b823cb9aa2",
                "type": "identity",
                "name": "keystone"
            }
        ],
        "links": {
            "self": "https://example.com/identity/v3/catalog",
            "previous": null,
            "next": null
        }
    }"#;

    mock_connector!(MockCatalog {
        "http://127.0.2.1" => String::from("HTTP/1.1 200 OK\r\n\
                                            Server: Mock.Mock\r\n\
                                            X-Subject-Token: abcdef\r\n
                                            \r\n") + EXAMPLE_CATALOG_RESPONSE
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
        assert_eq!(&token, "abcdef");
    }

    #[test]
    fn test_identity_get_token_unauthorized() {
        let id = Identity::new("http://127.0.1.2").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockToken::default());
        match id.get_token(&cli).err().unwrap() {
            ApiError::HttpError(StatusCode::Unauthorized, ..) => (),
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
            ApiError::HttpError(hyper::NotFound, ..) => (),
            other => panic!("Unexpected {}", other)
        };
    }

    fn get_endpoint(service_type: &str, interface_endpoint: Option<&str>,
                    region: Option<&str>) -> ApiResult<Url> {
        let id = Identity::new("http://127.0.2.1").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        let cli = hyper::Client::with_connector(MockCatalog::default());
        id.get_endpoint(&cli, String::from(service_type),
                        interface_endpoint.map(String::from),
                        region.map(String::from))
    }

    #[test]
    fn test_identity_get_endpoint() {
        let e1 = get_endpoint("identity", None, None).unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:5000/");
        let e2 = get_endpoint("identity", Some("admin"), None).unwrap();
        assert_eq!(&e2.to_string(), "http://localhost:35357/");

        match get_endpoint("foo", None, None).err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "foo"),
            other => panic!("Unexpected {}", other)
        };

        match get_endpoint("identity", Some("unknown"), None).err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };
    }

    #[test]
    fn test_identity_get_endpoint_with_region() {
        let e1 = get_endpoint("identity", Some("admin"),
                              Some("RegionOne")).unwrap();
        assert_eq!(&e1.to_string(), "http://localhost:35357/");

        match get_endpoint("identity", None,
                           Some("unknown")).err().unwrap() {
            ApiError::EndpointNotFound(ref endp) =>
                assert_eq!(endp, "identity"),
            other => panic!("Unexpected {}", other)
        };
    }
}
