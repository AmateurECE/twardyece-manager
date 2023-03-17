// Author: Ethan D. Twardy <ethan.twardy@gmail.com>
//
// Copyright 2023, Ethan Twardy. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the \"License\");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an \"AS IS\" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::Router;
use redfish_codegen::models::{odata_v4, resource};
use seuss::service;

mod endpoint;

#[tokio::main]
async fn main() {
    let service_root = endpoint::ServiceRoot::new(
        resource::Name("Basic Redfish Service".to_string()),
        resource::Id("example-basic".to_string()),
    );

    let systems = endpoint::Systems::new(
        odata_v4::Id("/redfish/v1/Systems".to_string()),
        vec![endpoint::DummySystem {
            odata_id: odata_v4::Id("/redfish/v1/Systems/1".to_string()),
            name: resource::Name("1".to_string()),
        }],
    );

    let app: Router = Router::new()
        .route(
            "/redfish/v1",
            service::ServiceRoot::new(service_root).into(),
        )
        .route(
            "/redfish/v1/Systems",
            service::Systems::new(systems.clone()).into(),
        )
        .route(
            "/redfish/v1/Systems/:name",
            service::ComputerSystemDetail::new(systems).into(),
        );
    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
