// Copyright 2016 Dmitry Tantsur <divius.inside@gmail.com>
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

use super::base::AuthMethod;


#[derive(RustcDecodable, RustcEncodable)]
struct Domain {
    name: String
}

#[derive(RustcDecodable, RustcEncodable)]
struct User {
    name: String,
    password: String,
    domain: Domain
}

#[derive(RustcDecodable, RustcEncodable)]
struct PasswordAuth {
    user: User
}


#[derive(RustcDecodable, RustcEncodable)]
struct Identity {
    methods: Vec<String>,
    password: PasswordAuth
}

#[derive(RustcDecodable, RustcEncodable)]
struct Project {
    name: String,
    domain: Domain
}

#[derive(RustcDecodable, RustcEncodable)]
struct ProjectScope {
    project: Project
}

#[derive(RustcDecodable, RustcEncodable)]
struct Auth {
    identity: Identity,
    scope: ProjectScope
}


/// Password authentication method.
pub struct Password {
    auth: Auth
}
