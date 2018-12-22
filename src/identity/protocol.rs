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

//! JSON structures and protocol bits for the Identity V3 API.

#![allow(missing_docs)]

use chrono::{DateTime, FixedOffset};
use super::super::common;


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Domain {
    pub name: String
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserAndPassword {
    pub name: String,
    pub password: String,
    pub domain: Domain
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PasswordAuth {
    pub user: UserAndPassword
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PasswordIdentity {
    pub methods: Vec<String>,
    pub password: PasswordAuth
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    pub domain: Domain
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectScope {
    pub project: Project
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectScopedAuth {
    pub identity: PasswordIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ProjectScope>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectScopedAuthRoot {
    pub auth: ProjectScopedAuth
}

#[derive(Clone, Debug, Deserialize)]
pub struct Endpoint {
    pub interface: String,
    pub region: String,
    pub url: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct CatalogRecord {
    #[serde(rename = "type")]
    pub service_type: String,
    pub endpoints: Vec<Endpoint>
}

#[derive(Clone, Debug, Deserialize)]
pub struct CatalogRoot {
    pub catalog: Vec<CatalogRecord>
}

#[derive(Clone, Debug, Deserialize)]
pub struct Token {
    pub roles: Vec<common::protocol::IdAndName>,
    pub expires_at: DateTime<FixedOffset>,
    pub catalog: Vec<CatalogRecord>
}

#[derive(Clone, Debug, Deserialize)]
pub struct TokenRoot {
    pub token: Token
}

const PASSWORD_METHOD: &str = "password";


impl PasswordAuth {
    fn new<S1, S2, S3>(user_name: S1, password: S2, domain_name: S3)
            -> PasswordAuth
            where S1: Into<String>, S2: Into<String>, S3: Into<String> {
        PasswordAuth {
            user: UserAndPassword {
                name: user_name.into(),
                password: password.into(),
                domain: Domain {
                    name: domain_name.into()
                }
            }
        }
    }
}

impl PasswordIdentity {
    pub fn new<S1, S2, S3>(user_name: S1, password: S2, domain_name: S3)
            -> PasswordIdentity
            where S1: Into<String>, S2: Into<String>, S3: Into<String> {
        PasswordIdentity {
            methods: vec![String::from(PASSWORD_METHOD)],
            password: PasswordAuth::new(user_name, password, domain_name)
        }
    }
}

impl ProjectScope {
    pub fn new<S1, S2>(project_name: S1, domain_name: S2) -> ProjectScope
            where S1: Into<String>, S2: Into<String> {
        ProjectScope {
            project: Project {
                name: project_name.into(),
                domain: Domain {
                    name: domain_name.into()
                }
            }
        }
    }
}

impl ProjectScopedAuthRoot {
    pub fn new(identity: PasswordIdentity, scope: Option<ProjectScope>)
            -> ProjectScopedAuthRoot {
        ProjectScopedAuthRoot {
            auth: ProjectScopedAuth {
                identity,
                scope,
            }
        }
    }
}
