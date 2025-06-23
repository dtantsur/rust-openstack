// Copyright 2019 Dmitry Tantsur <dtantsur@protonmail.com>
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

//! Identity V3 JSON structures and protocol bits.

use chrono::{DateTime, FixedOffset};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::IdOrName;

/// User and password.
#[derive(Clone, Debug, Serialize)]
pub struct UserAndPassword {
    #[serde(flatten)]
    pub user: IdOrName,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub domain: Option<IdOrName>,
}

/// Application credential.
#[derive(Clone, Debug, Serialize)]
pub struct ApplicationCredential {
    #[serde(flatten)]
    pub id: IdOrName,
    pub secret: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub user: Option<IdOrName>,
}

/// Authentication identity.
#[derive(Clone, Debug)]
pub enum Identity {
    /// Authentication with a user and a password.
    Password(UserAndPassword),
    /// Authentication with a token.
    Token(String),
    /// Authentication with an application credential.
    ApplicationCredential(ApplicationCredential),
}

/// A reference to a project in a domain.
#[derive(Clone, Debug, Serialize)]
pub struct Project {
    #[serde(flatten)]
    pub project: IdOrName,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub domain: Option<IdOrName>,
}

/// A scope.
#[allow(unused)]
#[derive(Clone, Debug, Serialize)]
pub enum Scope {
    /// Project scope.
    #[serde(rename = "project")]
    Project(Project),
    /// Domain scope.
    #[serde(rename = "domain")]
    Domain(IdOrName),
    #[serde(rename = "system", serialize_with = "ser_system_scope")]
    System,
}

/// An authentication object.
#[derive(Clone, Debug, Serialize)]
pub struct Auth {
    /// Authentication identity.
    pub identity: Identity,
    /// Authentication scope (if needed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
}

/// An authentication request root.
#[derive(Clone, Debug, Serialize)]
pub struct AuthRoot {
    pub auth: Auth,
}

/// An endpoint in the catalog.
#[derive(Clone, Debug, Deserialize)]
pub struct Endpoint {
    pub interface: String,
    #[allow(dead_code)] // FIXME(dtantsur): remove when migrated endpoint filters
    pub region: String,
    pub url: String,
}

/// A service catalog record.
#[derive(Clone, Debug, Deserialize)]
pub struct CatalogRecord {
    #[serde(rename = "type")]
    pub service_type: String,
    pub endpoints: Vec<Endpoint>,
}

/// An authentication token with embedded catalog.
#[derive(Clone, Debug, Deserialize)]
pub struct Token {
    pub expires_at: DateTime<FixedOffset>,
    pub catalog: Vec<CatalogRecord>,
}

/// A token response root.
#[derive(Clone, Debug, Deserialize)]
pub struct TokenRoot {
    pub token: Token,
}

#[derive(Debug, Serialize)]
struct PasswordAuth<'a> {
    user: &'a UserAndPassword,
}

#[derive(Debug, Serialize)]
struct TokenAuth<'a> {
    id: &'a str,
}

impl Serialize for Identity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut inner = serializer.serialize_struct("Identity", 2)?;
        match self {
            Identity::Password(ref user) => {
                inner.serialize_field("methods", &["password"])?;
                inner.serialize_field("password", &PasswordAuth { user })?;
            }
            Identity::Token(ref token) => {
                inner.serialize_field("methods", &["token"])?;
                inner.serialize_field("token", &TokenAuth { id: token })?;
            }
            Identity::ApplicationCredential(ref cred) => {
                inner.serialize_field("methods", &["application_credential"])?;
                inner.serialize_field("application_credential", &cred)?;
            }
        }
        inner.end()
    }
}

fn ser_system_scope<S>(serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut inner = serializer.serialize_struct("System", 1)?;
    inner.serialize_field("all", &true)?;
    inner.end()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::test;

    const PASSWORD_NAME_UNSCOPED: &str = r#"
{
    "auth": {
        "identity": {
            "methods": [
                "password"
            ],
            "password": {
                "user": {
                    "name": "admin",
                    "domain": {
                        "name": "Default"
                    },
                    "password": "devstacker"
                }
            }
        }
    }
}"#;

    const PASSWORD_ID_SCOPED_WITH_ID: &str = r#"
{
    "auth": {
        "identity": {
            "methods": [
                "password"
            ],
            "password": {
                "user": {
                    "id": "ee4dfb6e5540447cb3741905149d9b6e",
                    "password": "devstacker"
                }
            }
        },
        "scope": {
            "domain": {
                "id": "default"
            }
        }
    }
}"#;

    const PASSWORD_ID_SYSTEM_SCOPE: &str = r#"
{
    "auth": {
        "identity": {
            "methods": [
                "password"
            ],
            "password": {
                "user": {
                    "id": "ee4dfb6e5540447cb3741905149d9b6e",
                    "password": "devstacker"
                }
            }
        },
        "scope": {
            "system": {
                "all": true
            }
        }
    }
}"#;

    const TOKEN_SCOPED_WITH_NAME: &str = r#"
{
    "auth": {
        "identity": {
            "methods": [
                "token"
            ],
            "token": {
                "id": "abcdef"
            }
        },
        "scope": {
            "domain": {
                "name": "Default"
            }
        }
    }
}"#;

    const APPLICATION_CREDENTIAL_ID: &str = r#"
{
    "auth": {
        "identity": {
            "methods": [
                "application_credential"
            ],
            "application_credential": {
                "id": "abcdef",
                "secret": "shhhh"
            }
        }
    }
}"#;

    const APPLICATION_CREDENTIAL_NAME: &str = r#"
{
    "auth": {
        "identity": {
            "methods": [
                "application_credential"
            ],
            "application_credential": {
                "name": "abcdef",
                "secret": "shhhh",
                "user": {
                    "id": "a6b3c6e7a6d"
                }
            }
        }
    }
}"#;

    #[test]
    fn test_password_name_unscoped() {
        let value = AuthRoot {
            auth: Auth {
                identity: Identity::Password(UserAndPassword {
                    user: IdOrName::Name("admin".to_string()),
                    password: "devstacker".to_string(),
                    domain: Some(IdOrName::from_name("Default")),
                }),
                scope: None,
            },
        };
        test::compare(PASSWORD_NAME_UNSCOPED, value);
    }

    #[test]
    fn test_password_id_scoped_with_id() {
        let value = AuthRoot {
            auth: Auth {
                identity: Identity::Password(UserAndPassword {
                    user: IdOrName::Id("ee4dfb6e5540447cb3741905149d9b6e".to_string()),
                    password: "devstacker".to_string(),
                    domain: None,
                }),
                scope: Some(Scope::Domain(IdOrName::from_id("default"))),
            },
        };
        test::compare(PASSWORD_ID_SCOPED_WITH_ID, value);
    }

    #[test]
    fn test_password_id_system_scope() {
        let value = AuthRoot {
            auth: Auth {
                identity: Identity::Password(UserAndPassword {
                    user: IdOrName::Id("ee4dfb6e5540447cb3741905149d9b6e".to_string()),
                    password: "devstacker".to_string(),
                    domain: None,
                }),
                scope: Some(Scope::System),
            },
        };
        test::compare(PASSWORD_ID_SYSTEM_SCOPE, value);
    }

    #[test]
    fn test_token_scoped_with_name() {
        let value = AuthRoot {
            auth: Auth {
                identity: Identity::Token("abcdef".to_string()),
                scope: Some(Scope::Domain(IdOrName::Name("Default".to_string()))),
            },
        };
        test::compare(TOKEN_SCOPED_WITH_NAME, value);
    }

    #[test]
    fn test_application_credential_id() {
        let value = AuthRoot {
            auth: Auth {
                identity: Identity::ApplicationCredential(ApplicationCredential {
                    id: IdOrName::Id("abcdef".to_string()),
                    secret: "shhhh".to_string(),
                    user: None,
                }),
                scope: None,
            },
        };
        test::compare(APPLICATION_CREDENTIAL_ID, value);
    }

    #[test]
    fn test_application_credential_name() {
        let value = AuthRoot {
            auth: Auth {
                identity: Identity::ApplicationCredential(ApplicationCredential {
                    id: IdOrName::Name("abcdef".to_string()),
                    secret: "shhhh".to_string(),
                    user: Some(IdOrName::Id("a6b3c6e7a6d".into())),
                }),
                scope: None,
            },
        };
        test::compare(APPLICATION_CREDENTIAL_NAME, value);
    }
}
