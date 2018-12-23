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
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, IntoUrl, Method, RequestBuilder, Response, Url};

use super::super::identity::{catalog, protocol};
use super::super::session::RequestBuilderExt;
use super::super::utils::ValueCache;
use super::super::{Error, ErrorKind, Result};
use super::AuthMethod;

const MISSING_SUBJECT_HEADER: &str = "Missing X-Subject-Token header";
const INVALID_SUBJECT_HEADER: &str = "Invalid X-Subject-Token header";
// Required validity time in minutes. Here we refresh the token if it expires
// in 10 minutes or less.
const TOKEN_MIN_VALIDITY: i64 = 10;

/// Plain authentication token without additional details.
#[derive(Clone)]
struct Token {
    value: String,
    body: protocol::Token,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.value.hash(&mut hasher);
        write!(
            f,
            "Token {{ value: {}, body: {:?} }}",
            hasher.finish(),
            self.body
        )
    }
}

/// Generic trait for authentication using Identity API V3.
pub trait Identity {
    /// Get a reference to the auth URL.
    fn auth_url(&self) -> &Url;
}

/// Password authentication using Identity API V3.
#[derive(Clone, Debug)]
pub struct Password {
    client: Client,
    auth_url: Url,
    region: Option<String>,
    body: protocol::ProjectScopedAuthRoot,
    token_endpoint: String,
    cached_token: ValueCache<Token>,
}

impl Identity for Password {
    fn auth_url(&self) -> &Url {
        &self.auth_url
    }
}

impl Password {
    /// Create a password authentication against the given Identity service.
    pub fn new<U, S1, S2, S3>(
        auth_url: U,
        user_name: S1,
        password: S2,
        user_domain_name: S3,
    ) -> Result<Password>
    where
        U: IntoUrl,
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        Password::new_with_client(
            auth_url,
            Client::new(),
            user_name,
            password,
            user_domain_name,
        )
    }

    /// Create a password authentication against the given Identity service.
    pub fn new_with_client<U, S1, S2, S3>(
        auth_url: U,
        client: Client,
        user_name: S1,
        password: S2,
        user_domain_name: S3,
    ) -> Result<Password>
    where
        U: IntoUrl,
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        let url = auth_url.into_url()?;
        // TODO: more robust logic?
        let token_endpoint = if url.path().ends_with("/v3") {
            format!("{}/auth/tokens", url)
        } else {
            format!("{}/v3/auth/tokens", url)
        };
        let pw = protocol::PasswordIdentity::new(user_name, password, user_domain_name);
        let body = protocol::ProjectScopedAuthRoot::new(pw, None);
        Ok(Password {
            client,
            auth_url: url,
            region: None,
            body,
            token_endpoint,
            cached_token: ValueCache::new(None),
        })
    }

    /// User name.
    pub fn user_name(&self) -> &String {
        &self.body.auth.identity.password.user.name
    }

    /// Set a region for this authentication methjod.
    pub fn set_region<S>(&mut self, region: S)
    where
        S: Into<String>,
    {
        self.region = Some(region.into());
    }

    /// Scope authentication to the given project.
    ///
    /// This is required in the most cases.
    pub fn set_project_scope<S1, S2>(&mut self, project_name: S1, project_domain_name: S2)
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        self.body.auth.scope = Some(protocol::ProjectScope::new(
            project_name,
            project_domain_name,
        ));
    }

    /// Set a region for this authentication methjod.
    pub fn with_region<S>(mut self, region: S) -> Self
    where
        S: Into<String>,
    {
        self.set_region(region);
        self
    }

    /// Scope authentication to the given project.
    pub fn with_project_scope<S1, S2>(
        mut self,
        project_name: S1,
        project_domain_name: S2,
    ) -> Password
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        self.set_project_scope(project_name, project_domain_name);
        self
    }

    fn token_from_response(&self, mut resp: Response) -> Result<Token> {
        let value = match resp.headers().get("x-subject-token") {
            Some(hdr) => match hdr.to_str() {
                Ok(s) => s.to_string(),
                Err(e) => {
                    error!(
                        "Invalid X-Subject-Token received from {}: {}",
                        self.token_endpoint, e
                    );
                    return Err(Error::new(
                        ErrorKind::InvalidResponse,
                        INVALID_SUBJECT_HEADER,
                    ));
                }
            },
            None => {
                error!(
                    "No X-Subject-Token header received from {}",
                    self.token_endpoint
                );
                return Err(Error::new(
                    ErrorKind::InvalidResponse,
                    MISSING_SUBJECT_HEADER,
                ));
            }
        };

        let body = resp.json::<protocol::TokenRoot>()?.token;

        debug!(
            "Received a token for user {} from {} expiring at {}",
            self.user_name(),
            self.token_endpoint,
            body.expires_at
        );
        trace!("Received catalog: {:?}", body.catalog);

        Ok(Token { value, body })
    }

    fn refresh_token(&self) -> Result<()> {
        self.cached_token.validate_and_ensure_value(
            |val| {
                let validity_time_left = val.body.expires_at.signed_duration_since(Local::now());
                trace!("Token is valid for {:?}", validity_time_left);
                validity_time_left > Duration::minutes(TOKEN_MIN_VALIDITY)
            },
            || {
                debug!(
                    "Requesting a token for user {} from {}",
                    self.user_name(),
                    self.token_endpoint
                );
                let resp = self
                    .client
                    .post(&self.token_endpoint)
                    .json(&self.body)
                    .header(CONTENT_TYPE, "application/json")
                    .send_checked()?;
                self.token_from_response(resp)
            },
        )
    }

    fn get_token(&self) -> Result<String> {
        self.refresh_token()?;
        Ok(self.cached_token.extract(|t| t.value.clone()).unwrap())
    }

    fn get_catalog(&self) -> Result<Vec<protocol::CatalogRecord>> {
        self.refresh_token()?;
        Ok(self
            .cached_token
            .extract(|t| t.body.catalog.clone())
            .unwrap())
    }
}

impl AuthMethod for Password {
    /// Get region.
    fn region(&self) -> Option<String> {
        self.region.clone()
    }

    /// Create an authenticated request.
    fn request(&self, method: Method, url: Url) -> Result<RequestBuilder> {
        let token = self.get_token()?;
        let builder = self
            .client
            .request(method, url)
            .header("x-auth-token", token);
        Ok(builder)
    }

    /// Get a URL for the requested service.
    fn get_endpoint(
        &self,
        service_type: String,
        endpoint_interface: Option<String>,
    ) -> Result<Url> {
        let real_interface =
            endpoint_interface.unwrap_or_else(|| self.default_endpoint_interface());
        debug!(
            "Requesting a catalog endpoint for service '{}', interface \
             '{}' from region {:?}",
            service_type, real_interface, self.region
        );
        let cat = self.get_catalog()?;
        let endp = catalog::find_endpoint(&cat, &service_type, &real_interface, &self.region)?;
        debug!("Received {:?} for {}", endp, service_type);
        Url::parse(&endp.url).map_err(|e| {
            error!(
                "Invalid URL {} received from service catalog for service \
                 '{}', interface '{}' from region {:?}: {}",
                endp.url, service_type, real_interface, self.region, e
            );
            Error::new(
                ErrorKind::InvalidResponse,
                format!("Invalid URL {} for {} - {}", endp.url, service_type, e),
            )
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
    use super::{Identity, Password};

    #[test]
    fn test_identity_new() {
        let id = Password::new("http://127.0.0.1:8080/", "admin", "pa$$w0rd", "Default").unwrap();
        let e = id.auth_url();
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/");
        assert_eq!(id.user_name(), "admin");
    }

    #[test]
    fn test_identity_new_invalid() {
        Password::new("http://127.0.0.1 8080/", "admin", "pa$$w0rd", "Default")
            .err()
            .unwrap();
    }

    #[test]
    fn test_identity_create() {
        let id = Password::new(
            "http://127.0.0.1:8080/identity",
            "user",
            "pa$$w0rd",
            "example.com",
        )
        .unwrap()
        .with_project_scope("cool project", "example.com");
        assert_eq!(id.auth_url().to_string(), "http://127.0.0.1:8080/identity");
        assert_eq!(&id.body.auth.identity.password.user.name, "user");
        assert_eq!(&id.body.auth.identity.password.user.password, "pa$$w0rd");
        assert_eq!(
            &id.body.auth.identity.password.user.domain.name,
            "example.com"
        );
        assert_eq!(
            id.body.auth.identity.methods,
            vec![String::from("password")]
        );
        assert_eq!(
            &id.body.auth.scope.as_ref().unwrap().project.name,
            "cool project"
        );
        assert_eq!(
            &id.body.auth.scope.as_ref().unwrap().project.domain.name,
            "example.com"
        );
        assert_eq!(
            &id.token_endpoint,
            "http://127.0.0.1:8080/identity/v3/auth/tokens"
        );
        assert_eq!(id.region(), None);
    }
}
