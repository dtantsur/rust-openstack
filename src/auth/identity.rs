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
use std::fmt;
use std::hash::{Hash, Hasher};

use chrono::{Duration, Local};
use reqwest::{Client, IntoUrl, Method, Response, StatusCode, Url, UrlError};
use reqwest::header::{ContentType, Headers};

use super::super::{Error, ErrorKind, Result};
use super::super::identity::{catalog, protocol};
use super::super::session::RequestBuilder;
use super::super::utils::ValueCache;
use super::AuthMethod;


const MISSING_USER: &'static str = "User information required";
const MISSING_SCOPE: &'static str = "Unscoped tokens are not supported now";
const MISSING_SUBJECT_HEADER: &'static str =
    "Missing X-Subject-Token header";
// Required validity time in minutes. Here we refresh the token if it expires
// in 10 minutes or less.
const TOKEN_MIN_VALIDITY: i64 = 10;


/// Plain authentication token without additional details.
#[derive(Clone)]
struct Token {
    value: String,
    body: protocol::Token
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.value.hash(&mut hasher);
        write!(f, "Token {{ value: {}, body: {:?} }}",
               hasher.finish(), self.body)
    }
}


/// Authentication method factory using Identity API V3.
#[derive(Clone, Debug)]
pub struct Identity {
    client: Client,
    auth_url: Url,
    region: Option<String>,
    password_identity: Option<protocol::PasswordIdentity>,
    project_scope: Option<protocol::ProjectScope>
}

/// Password authentication using Identity API V3.
///
/// Has to be created via [Identity object](struct.Identity.html) methods.
#[derive(Clone, Debug)]
pub struct PasswordAuth {
    client: Client,
    auth_url: Url,
    region: Option<String>,
    body: protocol::ProjectScopedAuthRoot,
    token_endpoint: String,
    cached_token: ValueCache<Token>
}

impl Identity {
    /// Get a reference to the auth URL.
    pub fn auth_url(&self) -> &Url {
        &self.auth_url
    }

    /// Create a password authentication against the given Identity service.
    pub fn new<U>(auth_url: U) -> ::std::result::Result<Identity, UrlError>
            where U: IntoUrl  {
        Identity::new_with_client(auth_url, Client::new())
    }

    /// Create a password authentication against the given Identity service.
    pub fn new_with_region<U>(auth_url: U, region: String)
            -> ::std::result::Result<Identity, UrlError> where U: IntoUrl  {
        Ok(Identity {
            client: Client::new(),
            auth_url: auth_url.into_url()?,
            region: Some(region),
            password_identity: None,
            project_scope: None,
        })
    }

    /// Create a password authentication against the given Identity service.
    pub fn new_with_client<U>(auth_url: U, client: Client)
            -> ::std::result::Result<Identity, UrlError> where U: IntoUrl  {
        Ok(Identity {
            client: client,
            auth_url: auth_url.into_url()?,
            region: None,
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
    pub fn create(self) -> Result<PasswordAuth> {
        // TODO: support more authentication methods (at least a token)
        let password_identity = match self.password_identity {
            Some(p) => p,
            None =>
                return Err(Error::new(ErrorKind::InvalidInput, MISSING_USER))
        };

        // TODO: support unscoped tokens
        let project_scope = match self.project_scope {
            Some(p) => p,
            None =>
                return Err(Error::new(ErrorKind::InvalidInput, MISSING_SCOPE))
        };

        Ok(PasswordAuth::new(self.auth_url, self.region, password_identity,
                             project_scope, self.client))
    }
}

#[inline]
fn extract_subject_token(headers: &Headers) -> Option<String> {
    // TODO: replace with a typed header
    headers.get_raw("x-subject-token").and_then(|h| h.one())
        .map(|buf| { String::from_utf8_lossy(buf).into_owned() })
}

impl PasswordAuth {
    /// Get a reference to the auth URL.
    pub fn auth_url(&self) -> &Url {
        &self.auth_url
    }

    fn new(auth_url: Url, region: Option<String>,
           password_identity: protocol::PasswordIdentity,
           project_scope: protocol::ProjectScope,
           client: Client) -> PasswordAuth {
        let body = protocol::ProjectScopedAuthRoot::new(password_identity,
                                                        project_scope);
        // TODO: more robust logic?
        let token_endpoint = if auth_url.path().ends_with("/v3") {
            format!("{}/auth/tokens", auth_url)
        } else {
            format!("{}/v3/auth/tokens", auth_url)
        };

        PasswordAuth {
            client: client,
            auth_url: auth_url,
            region: region,
            body: body,
            token_endpoint: token_endpoint,
            cached_token: ValueCache::new(None)
        }
    }

    fn token_from_response(&self, mut resp: Response) -> Result<Token> {
        let token_value = match resp.status() {
            StatusCode::Ok | StatusCode::Created => {
                match extract_subject_token(resp.headers()) {
                    Some(value) => value,
                    None => {
                        error!("No X-Subject-Token header received from {}",
                               self.token_endpoint);
                        return Err(Error::new(ErrorKind::InvalidResponse,
                                              MISSING_SUBJECT_HEADER));
                    }
                }
            },
            StatusCode::Unauthorized => {
                error!("Invalid credentials for user {}",
                       self.body.auth.identity.password.user.name);
                return Err(Error::new_with_details(
                    ErrorKind::AuthenticationFailed,
                    Some(resp.status()),
                    Some(String::from("Unable to authenticate"))
                ));
            },
            other => {
                error!("Unexpected HTTP error {} when getting a token for {}",
                       other, self.body.auth.identity.password.user.name);
                return Err(Error::new_with_details(
                    ErrorKind::AuthenticationFailed,
                    Some(resp.status()),
                    Some(format!("Unexpected HTTP code {} when authenticating",
                                 resp.status()))
                ));
            }
        };

        let body = resp.json::<protocol::TokenRoot>()?.token;

        debug!("Received a token for user {} from {} expiring at {}",
               self.body.auth.identity.password.user.name,
               self.token_endpoint, body.expires_at);
        trace!("Received catalog: {:?}", body.catalog);

        Ok(Token {
            value: token_value,
            body: body
        })
    }

    fn refresh_token(&self) -> Result<()> {
        self.cached_token.validate_and_ensure_value(|val| {
            let validity_time_left = val.body.expires_at.clone()
                .signed_duration_since(Local::now());
            trace!("Token is valid for {:?}", validity_time_left);
            return validity_time_left > Duration::minutes(TOKEN_MIN_VALIDITY);
        }, || {
            debug!("Requesting a token for user {} from {}",
                   self.body.auth.identity.password.user.name,
                   self.token_endpoint);
            let resp = self.client.post(&self.token_endpoint).json(&self.body)
                .header(ContentType::json()).send()?.error_for_status()?;
            self.token_from_response(resp)
        })
    }

    fn get_token(&self) -> Result<String> {
        self.refresh_token()?;
        Ok(self.cached_token.extract(|t| t.value.clone()).unwrap())
    }

    fn get_catalog(&self) -> Result<Vec<protocol::CatalogRecord>> {
        self.refresh_token()?;
        Ok(self.cached_token.extract(|t| t.body.catalog.clone()).unwrap())
    }
}

impl AuthMethod for PasswordAuth {
    /// Get region.
    fn region(&self) -> Option<String> { self.region.clone() }

    /// Create an authenticated request.
    fn request(&self, method: Method, url: Url) -> Result<RequestBuilder> {
        let token = self.get_token()?;
        let mut headers = Headers::new();
        // TODO: replace with a typed header
        headers.set_raw("x-auth-token", token);
        let mut builder = self.client.request(method, url);
        {
            let _unused = builder.headers(headers);
        }
        Ok(RequestBuilder::new(builder))
    }

    /// Get a URL for the requested service.
    fn get_endpoint(&self, service_type: String,
                    endpoint_interface: Option<String>) -> Result<Url> {
        let real_interface = endpoint_interface.unwrap_or(
            self.default_endpoint_interface());
        debug!("Requesting a catalog endpoint for service '{}', interface \
               '{}' from region {:?}", service_type, real_interface,
               self.region);
        let cat = self.get_catalog()?;
        let endp = catalog::find_endpoint(&cat, &service_type,
                                          &real_interface,
                                          &self.region)?;
        debug!("Received {:?} for {}", endp, service_type);
        Url::parse(&endp.url).map_err(|e| {
            error!("Invalid URL {} received from service catalog for service \
                   '{}', interface '{}' from region {:?}: {}",
                   endp.url, service_type, real_interface, self.region, e);
            Error::new(ErrorKind::InvalidResponse,
                       format!("Invalid URL {} for {} - {}",
                               endp.url, service_type, e))
        })
    }

    fn refresh(&mut self) -> Result<()> {
        self.cached_token = ValueCache::new(None);
        self.refresh_token()
    }
}

#[cfg(test)]
pub mod test {
    #![allow(unused_results)]

    use super::super::AuthMethod;
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

    #[test]
    fn test_identity_create() {
        let id = Identity::new("http://127.0.0.1:8080/identity").unwrap()
            .with_user("user", "pa$$w0rd", "example.com")
            .with_project_scope("cool project", "example.com")
            .create().unwrap();
        assert_eq!(&id.auth_url.to_string(), "http://127.0.0.1:8080/identity");
        assert_eq!(id.auth_url().to_string(),
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
        assert_eq!(id.region(), None);
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
}
