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

use std::env;
use waiter::Waiter;

#[cfg(feature = "compute")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let id = env::args().nth(1).expect("Provide a server ID");
    let action = env::args().nth(2).expect("Provide an action");

    let mut server = os.get_server(id).await.expect("Cannot get a server");
    match action.as_ref() {
        "start" => server
            .start()
            .await
            .expect("Cannot power on the server")
            .wait()
            .await
            .expect("Failed to reach ACTIVE"),
        "stop" => server
            .stop()
            .await
            .expect("Cannot power off the server")
            .wait()
            .await
            .expect("Failed to reach SHUTOFF"),
        "reboot" => server
            .reboot(openstack::compute::RebootType::Soft)
            .await
            .expect("Cannot reboot the server")
            .wait()
            .await
            .expect("Failed to reach ACTIVE"),
        "delete" => server
            .delete()
            .await
            .expect("Cannot delete the server")
            .wait()
            .await
            .expect("Failed to delete the server"),
        _ => panic!("Unknown action, supported are 'start' and 'stop'"),
    }
}

#[cfg(not(feature = "compute"))]
fn main() {
    panic!("This example cannot run with 'compute' feature disabled");
}
