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

use axum::{response::Redirect, Router};
use clap::Parser;
use redfish_codegen::models::{odata_v4, resource};
use seuss::{
    auth::{pam::LinuxPamAuthenticator, CombinedAuthenticationProxy, Role},
    routing,
    service::{self, session_manager::InMemorySessionManager},
};
use std::{collections::HashMap, fs::File};
use tower_http::trace::TraceLayer;

mod auth;
mod endpoint;

#[derive(Parser)]
struct Args {
    /// Configuration file
    #[clap(value_parser, short, long)]
    config: String,
}

#[derive(serde::Deserialize)]
struct Configuration {
    #[serde(rename = "role-map")]
    role_map: HashMap<Role, String>,
    server: redfish_service::Configuration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config: Configuration = serde_yaml::from_reader(File::open(&args.config)?)?;

    let sessions: &'static str = "/redfish/v1/SessionService/Sessions";

    let service_root = endpoint::ServiceRoot::new(
        resource::Name("Basic Redfish Service".to_string()),
        resource::Id("example-basic".to_string()),
    )
    .enable_systems()
    .enable_sessions(odata_v4::Id(sessions.to_string()));

    let service_document = routing::OData::new()
        .enable_systems()
        .enable_session_service()
        .enable_sessions();

    let authenticator = LinuxPamAuthenticator::new(config.role_map)?;
    let session_collection =
        InMemorySessionManager::new(authenticator.clone(), odata_v4::Id(sessions.to_string()));
    let proxy = CombinedAuthenticationProxy::new(session_collection.clone(), authenticator);

    let systems = endpoint::Systems::new(
        odata_v4::Id("/redfish/v1/Systems".to_string()),
        resource::Name("Computer System Collection".to_string()),
        vec![endpoint::DummySystem {
            odata_id: odata_v4::Id("/redfish/v1/Systems/1".to_string()),
            name: resource::Name("1".to_string()),
            ..Default::default()
        }],
        proxy.clone(),
    );

    let app: Router = Router::new()
        .route("/redfish", routing::RedfishVersions::default().into())
        .route(
            "/redfish/v1",
            axum::routing::get(|| async { Redirect::permanent("/redfish/v1/") }),
        )
        .route(
            "/redfish/v1/",
            routing::ServiceRoot::new(service_root).into(),
        )
        .route("/redfish/v1/odata", service_document.into())
        .route("/redfish/v1/$metadata", routing::Metadata.into())
        .route(
            "/redfish/v1/Systems",
            routing::Systems::new(systems.clone()).into(),
        )
        .route(
            "/redfish/v1/Systems/:name",
            routing::computer_system_detail::ComputerSystemDetail::new(systems.clone()).into(),
        )
        .route(
            "/redfish/v1/Systems/:name/Actions/ComputerSystem.Reset",
            routing::computer_system_detail::reset::ResetRouter::new(systems).into(),
        )
        .route(
            "/redfish/v1/SessionService",
            routing::SessionService::new(service::SessionService::new(
                odata_v4::Id("/redfish/v1/SessionService".to_string()),
                resource::Name("Stub Session Service".to_string()),
                odata_v4::Id(sessions.to_string()),
                proxy.clone(),
            ))
            .into(),
        )
        .route(
            sessions,
            routing::sessions::Sessions::new(service::SessionCollection::new(
                odata_v4::Id(sessions.to_string()),
                resource::Name("Session Collection".to_string()),
                proxy,
                session_collection.clone(),
            ))
            .into(),
        )
        .layer(TraceLayer::new_for_http());

    redfish_service::serve(config.server, app).await
}
