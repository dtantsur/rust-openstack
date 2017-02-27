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

//! Various utilities.

use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;


/// Create an HTTP(s) client.
pub fn http_client() -> Client {
    let connector = HttpsConnector::new(TlsClient::new());
    Client::with_connector(connector)
}
