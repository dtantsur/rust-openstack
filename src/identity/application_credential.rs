// Copyright 2023 Matt Williams <matt@milliams.com>
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

//! Application Credential authentication.

use async_trait::async_trait;
use osauth::{AuthType, EndpointFilters, Error};
use reqwest::{Client, RequestBuilder, Url};
use static_assertions::assert_impl_all;

use super::internal::Internal;
use super::protocol;
use super::IdOrName;

/// Application Credential authentication using Identity API V3.
///
/// For any Identity authentication you need to know `auth_url`, which is an authentication endpoint
/// of the Identity service. For the Application Credential authentication you also need:
/// 1. Application Credential ID
/// 2. Application Credential secret
///
/// Start with creating a `ApplicationCredential` object using [new](#method.new):
///
/// ```rust,no_run
/// use osauth::common::IdOrName;
/// let auth = osauth::identity::ApplicationCredential::new(
///     "https://cloud.local/identity",
///     "<a cred id>",
///     "<a cred secret>",
/// )
/// .expect("Invalid auth_url");
///
/// let session = osauth::Session::new(auth);
/// ```
///
/// The authentication token is cached while it's still valid or until
/// [refresh](../trait.AuthType.html#tymethod.refresh) is called.
/// Clones of an `ApplicationCredential` also start with an empty cache.
#[derive(Debug, Clone)]
pub struct ApplicationCredential {
    inner: Internal,
}

assert_impl_all!(ApplicationCredential: Send, Sync);

impl ApplicationCredential {
    /// Create an application credential authentication.
    pub fn new<U, S1, S2>(auth_url: U, id: S1, secret: S2) -> Result<Self, Error>
    where
        U: AsRef<str>,
        S1: Into<String>,
        S2: Into<String>,
    {
        let app_cred = protocol::ApplicationCredential {
            id: IdOrName::Id(id.into()),
            secret: secret.into(),
            user: None,
        };
        let body = protocol::AuthRoot {
            auth: protocol::Auth {
                identity: protocol::Identity::ApplicationCredential(app_cred),
                scope: None,
            },
        };
        Ok(Self {
            inner: Internal::new(auth_url.as_ref(), body)?,
        })
    }

    /// Create an application credential authentication from a credential name.
    pub fn with_user_id<U, S1, S2, S3>(
        auth_url: U,
        name: S1,
        secret: S2,
        user_id: S3,
    ) -> Result<Self, Error>
    where
        U: AsRef<str>,
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        let app_cred = protocol::ApplicationCredential {
            id: IdOrName::Name(name.into()),
            secret: secret.into(),
            user: Some(IdOrName::Id(user_id.into())),
        };
        let body = protocol::AuthRoot {
            auth: protocol::Auth {
                identity: protocol::Identity::ApplicationCredential(app_cred),
                scope: None,
            },
        };
        Ok(Self {
            inner: Internal::new(auth_url.as_ref(), body)?,
        })
    }

    /// Project name or ID (if project scoped).
    #[inline]
    pub fn project(&self) -> Option<&IdOrName> {
        self.inner.project()
    }
}

#[async_trait]
impl AuthType for ApplicationCredential {
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

    use super::ApplicationCredential;

    #[test]
    fn test_identity_new() {
        let id = ApplicationCredential::new("http://127.0.0.1:8080/", "abcdef", "shhhh").unwrap();
        let e = Url::parse(id.inner.token_endpoint()).unwrap();
        assert_eq!(e.scheme(), "http");
        assert_eq!(e.host_str().unwrap(), "127.0.0.1");
        assert_eq!(e.port().unwrap(), 8080u16);
        assert_eq!(e.path(), "/v3/auth/tokens");
    }
}
