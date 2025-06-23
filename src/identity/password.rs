// Copyright 2019-2020 Dmitry Tantsur <dtantsur@protonmail.com>
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

//! Password authentication.

use async_trait::async_trait;
use osauth::{AuthType, Error};
use reqwest::{Client, RequestBuilder, Url};
use static_assertions::assert_impl_all;

use super::internal::Internal;
use super::protocol;
use super::Scope;
use crate::{EndpointFilters, IdOrName};

/// Password authentication using Identity API V3.
///
/// For any Identity authentication you need to know `auth_url`, which is an authentication endpoint
/// of the Identity service. For the Password authentication you also need:
/// 1. User name and password.
/// 2. Domain of the user.
/// 3. Name of the project to use.
/// 4. Domain of the project.
///
/// Note: currently only names are supported for user, user domain and project domain. ID support is
/// coming later.
///
/// Start with creating a `Password` object using [new](#method.new), then add a project scope
/// with [with_project_scope](#method.with_project_scope):
///
/// ```rust,no_run
/// # async fn example() -> Result<(), osauth::Error> {
/// use osauth::common::IdOrName;
/// let auth = osauth::identity::Password::new(
///     "https://cloud.local/identity",
///     "admin",
///     "pa$$w0rd",
///     "Default"
/// )?
/// .with_project_scope(IdOrName::from_name("project1"), IdOrName::from_id("default"));
///
/// let session = osauth::Session::new(auth).await?;
/// # Ok(()) }
/// # #[tokio::main]
/// # async fn main() { example().await.unwrap(); }
/// ```
///
/// By default, the `public` endpoint interface is used. If you would prefer to default to another
/// one, you can set it on the `Session`. Region can also be set there:
///
/// ```rust,no_run
/// # async fn example() -> Result<(), osauth::Error> {
/// use osauth::common::IdOrName;
///
/// let scope = osauth::identity::Scope::Project {
///     project: IdOrName::from_name("project1"),
///     domain: Some(IdOrName::from_id("default")),
/// };
/// let auth = osauth::identity::Password::new(
///     "https://cloud.local/identity",
///     "admin",
///     "pa$$w0rd",
///     "Default"
/// )?
/// .with_scope(scope);
///
/// let session = osauth::Session::new(auth)
///     .await?
///     .with_endpoint_interface(osauth::InterfaceType::Internal)
///     .with_region("US-East");
/// # Ok(()) }
/// # #[tokio::main]
/// # async fn main() { example().await.unwrap(); }
/// ```
///
/// The authentication token is cached while it's still valid or until
/// [refresh](../trait.AuthType.html#tymethod.refresh) is called.
/// Clones of a `Password` also start with an empty cache.
#[derive(Debug, Clone)]
pub struct Password {
    inner: Internal,
}

assert_impl_all!(Password: Send, Sync);

impl Password {
    /// Create a password authentication.
    pub fn new<U, S1, S2, S3>(
        auth_url: U,
        user_name: S1,
        password: S2,
        user_domain_name: S3,
    ) -> Result<Password, Error>
    where
        U: AsRef<str>,
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        let pw = protocol::UserAndPassword {
            user: IdOrName::Name(user_name.into()),
            password: password.into(),
            domain: Some(IdOrName::Name(user_domain_name.into())),
        };
        let body = protocol::AuthRoot {
            auth: protocol::Auth {
                identity: protocol::Identity::Password(pw),
                scope: None,
            },
        };
        Ok(Password {
            inner: Internal::new(auth_url.as_ref(), body)?,
        })
    }

    /// Scope authentication to the given project.
    ///
    /// A convenience wrapper around `set_scope`.
    #[inline]
    pub fn set_project_scope(&mut self, project: IdOrName, domain: impl Into<Option<IdOrName>>) {
        self.set_scope(Scope::Project {
            project,
            domain: domain.into(),
        });
    }

    /// Add a scope to the authentication.
    ///
    /// This is required in the most cases.
    #[inline]
    pub fn set_scope(&mut self, scope: Scope) {
        self.inner.set_scope(scope);
    }

    /// Scope authentication to the given project.
    ///
    /// A convenience wrapper around `with_scope`.
    #[inline]
    pub fn with_project_scope(
        mut self,
        project: IdOrName,
        domain: impl Into<Option<IdOrName>>,
    ) -> Password {
        self.set_project_scope(project, domain);
        self
    }

    /// Add a scope to the authentication.
    #[inline]
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.set_scope(scope);
        self
    }

    /// User name or ID.
    #[inline]
    pub fn user(&self) -> &IdOrName {
        self.inner.user().expect("Password auth without a user")
    }

    /// Project name or ID (if project scoped).
    #[inline]
    pub fn project(&self) -> Option<&IdOrName> {
        self.inner.project()
    }
}

#[async_trait]
impl AuthType for Password {
    /// Authenticate a request.
    async fn authenticate(
        &self,
        client: &Client,
        request: RequestBuilder,
    ) -> Result<RequestBuilder, Error> {
        self.inner.authenticate(client, request).await
    }

    /// Get a URL for the requested service.
    async fn get_endpoint(
        &self,
        client: &Client,
        service_type: &str,
        filters: &EndpointFilters,
    ) -> Result<Url, Error> {
        self.inner.get_endpoint(client, service_type, filters).await
    }

    /// Refresh the cached token and service catalog.
    async fn refresh(&self, client: &Client) -> Result<(), Error> {
        self.inner.refresh(client, true).await
    }
}

#[cfg(test)]
pub mod test {
    #![allow(unused_results)]

    use reqwest::Url;

    use super::Password;
    use crate::identity::IdOrName;

    #[test]
    fn test_identity_new() {
        let id = Password::new("http://127.0.0.1:8080/", "admin", "pa$$w0rd", "Default").unwrap();
        let e = Url::parse(id.inner.token_endpoint()).unwrap();
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/v3/auth/tokens");
        assert_eq!(id.user(), &IdOrName::Name("admin".to_string()));
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
        .with_project_scope(
            IdOrName::Name("cool project".to_string()),
            IdOrName::Name("example.com".to_string()),
        );
        assert_eq!(id.user(), &IdOrName::Name("user".to_string()));
        assert_eq!(
            id.project(),
            Some(&IdOrName::Name("cool project".to_string()))
        );
        assert_eq!(
            id.inner.token_endpoint(),
            "http://127.0.0.1:8080/identity/v3/auth/tokens"
        );
    }

    #[test]
    fn test_token_endpoint_with_trailing_slash() {
        let id = Password::new(
            "http://127.0.0.1:8080/identity/",
            "user",
            "pa$$w0rd",
            "example.com",
        )
        .unwrap()
        .with_project_scope(
            IdOrName::Name("cool project".to_string()),
            IdOrName::Name("example.com".to_string()),
        );
        assert_eq!(id.user(), &IdOrName::Name("user".to_string()));
        assert_eq!(
            id.project(),
            Some(&IdOrName::Name("cool project".to_string()))
        );
        assert_eq!(
            id.inner.token_endpoint(),
            "http://127.0.0.1:8080/identity/v3/auth/tokens"
        );
    }

    #[test]
    fn test_token_endpoint_with_v3() {
        let id = Password::new(
            "http://127.0.0.1:8080/identity/v3",
            "user",
            "pa$$w0rd",
            "example.com",
        )
        .unwrap()
        .with_project_scope(
            IdOrName::Name("cool project".to_string()),
            IdOrName::Name("example.com".to_string()),
        );
        assert_eq!(id.user(), &IdOrName::Name("user".to_string()));
        assert_eq!(
            id.project(),
            Some(&IdOrName::Name("cool project".to_string()))
        );
        assert_eq!(
            id.inner.token_endpoint(),
            "http://127.0.0.1:8080/identity/v3/auth/tokens"
        );
    }

    #[test]
    fn test_token_endpoint_with_trailing_slash_v3() {
        let id = Password::new(
            "http://127.0.0.1:8080/identity/v3/",
            "user",
            "pa$$w0rd",
            "example.com",
        )
        .unwrap()
        .with_project_scope(
            IdOrName::Name("cool project".to_string()),
            IdOrName::Name("example.com".to_string()),
        );
        assert_eq!(id.user(), &IdOrName::Name("user".to_string()));
        assert_eq!(
            id.project(),
            Some(&IdOrName::Name("cool project".to_string()))
        );
        assert_eq!(
            id.inner.token_endpoint(),
            "http://127.0.0.1:8080/identity/v3/auth/tokens"
        );
    }

    #[test]
    fn test_token_endpoint_root() {
        let id = Password::new("http://127.0.0.1:8080", "user", "pa$$w0rd", "example.com")
            .unwrap()
            .with_project_scope(
                IdOrName::Name("cool project".to_string()),
                IdOrName::Name("example.com".to_string()),
            );
        assert_eq!(id.user(), &IdOrName::Name("user".to_string()));
        assert_eq!(
            id.project(),
            Some(&IdOrName::Name("cool project".to_string()))
        );
        assert_eq!(
            id.inner.token_endpoint(),
            "http://127.0.0.1:8080/v3/auth/tokens"
        );
    }
}
