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
use clap::Parser;
use redfish_codegen::models::{odata_v4, resource};
use seuss::{
    auth::{BasicAuthenticationProxy, Role},
    service,
};
use seuss_auth_pam::LinuxPamBasicAuthenticator;
use std::collections::HashMap;
use std::fs::File;
use tower_http::trace::TraceLayer;

mod auth;
mod endpoint;

#[derive(serde::Deserialize)]
struct Configuration {
    #[serde(rename = "role-map")]
    role_map: HashMap<Role, String>,
    address: String,
}

#[derive(Parser)]
struct Args {
    /// Configuration file
    #[clap(value_parser, short, long)]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config: Configuration = serde_yaml::from_reader(File::open(&args.config)?)?;

    let service_root = endpoint::ServiceRoot::new(
        resource::Name("Basic Redfish Service".to_string()),
        resource::Id("example-basic".to_string()),
    );

    let systems = endpoint::Systems::new(
        odata_v4::Id("/redfish/v1/Systems".to_string()),
        vec![endpoint::DummySystem {
            odata_id: odata_v4::Id("/redfish/v1/Systems/1".to_string()),
            name: resource::Name("1".to_string()),
            ..Default::default()
        }],
        BasicAuthenticationProxy::new(LinuxPamBasicAuthenticator::new(config.role_map)?),
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
            service::computer_system_detail::ComputerSystemDetail::new(systems.clone()).into(),
        )
        .route(
            "/redfish/v1/Systems/:name/Actions/ComputerSystem.Reset",
            service::computer_system_detail::reset::ResetRouter::new(systems).into(),
        )
        .layer(TraceLayer::new_for_http());
    axum::Server::bind(&config.address.parse().unwrap())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
