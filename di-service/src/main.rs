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

use std::{
    collections::HashMap, fs::File, future::Future, num::ParseIntError, pin::Pin, task::Poll,
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use clap::Parser;
use redfish_codegen::models::{
    computer_system::v1_20_0::ComputerSystem as System,
    computer_system_collection::ComputerSystemCollection as Model, redfish,
};
use seuss::auth::Role;

mod computer_system_collection;

use computer_system_collection::{ComputerSystem, ComputerSystemCollection};
use tower::Service;
use tower_http::trace::TraceLayer;
use tracing::{event, Level};

use crate::computer_system_collection::redfish_map_err;

#[derive(Parser)]
struct Args {
    /// Configuration file
    #[clap(value_parser, short, long)]
    config: String,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct Configuration {
    #[serde(rename = "role-map")]
    role_map: HashMap<Role, String>,
    server: redfish_service::Configuration,
}

#[derive(Clone, Default)]
struct ResourceLocator;
impl Service<Path<HashMap<String, String>>> for ResourceLocator {
    type Response = u32;
    type Error = ParseIntError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, parameters: Path<HashMap<String, String>>) -> Self::Future {
        let id = parameters.get("computer_system").unwrap().clone();
        let result = u32::from_str_radix(id.as_str(), 10);
        let future = async { result };
        Box::pin(future)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let config: Configuration = serde_yaml::from_reader(File::open(&args.config)?)?;
    let app =
        Router::new()
            .nest(
                "/redfish/v1/Systems",
                ComputerSystemCollection::default()
                    .read(|| async {
                        let model = Model::default();
                        Json(model)
                    })
                    .create(|Json(model): Json<Model>| async {
                        event!(
                            Level::INFO,
                            "{}",
                            &serde_json::to_string(&model).map_err(redfish_map_err)?
                        );
                        Ok::<_, (StatusCode, Json<redfish::Error>)>(Json(model))
                    })
                    .systems(
                        ComputerSystem::default().replace(
                            |State(_locator): State<ResourceLocator>,
                             Json(system): Json<System>| async move {
                                event!(
                                    Level::INFO,
                                    "{}",
                                    &serde_json::to_string(&system).map_err(redfish_map_err)?
                                );
                                Ok::<_, (StatusCode, Json<redfish::Error>)>(Json(system))
                            },
                        ),
                        ResourceLocator,
                    )
                    .into(),
            )
            .layer(TraceLayer::new_for_http());

    redfish_service::serve(config.server, app).await
}
