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

use std::{collections::HashMap, fs::File};

use axum::{
    body::Body,
    extract::{FromRequestParts, Path},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    Extension, Json, Router,
};
use clap::Parser;
use redfish_codegen::models::{
    computer_system::v1_20_0::ComputerSystem as System,
    computer_system_collection::ComputerSystemCollection as Model, redfish,
};
use seuss::auth::Role;

mod computer_system_collection;

use computer_system_collection::{ComputerSystem, ComputerSystemCollection};
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let config: Configuration = serde_yaml::from_reader(File::open(&args.config)?)?;
    let app = Router::new()
        .nest(
            "/redfish/v1/Systems",
            ComputerSystemCollection::default()
                .get(|| async {
                    let model = Model::default();
                    Json(model)
                })
                .post(|Json(model): Json<Model>| async {
                    event!(
                        Level::INFO,
                        "{}",
                        &serde_json::to_string(&model).map_err(redfish_map_err)?
                    );
                    Ok::<_, (StatusCode, Json<redfish::Error>)>(Json(model))
                })
                .systems(
                    ComputerSystem::default()
                        .put(
                            |Extension(id): Extension<u32>, Json(system): Json<System>| async move {
                                event!(
                                    Level::INFO,
                                    "id={}, body={}",
                                    id,
                                    &serde_json::to_string(&system).map_err(redfish_map_err)?
                                );
                                Ok::<_, (StatusCode, Json<redfish::Error>)>(Json(system))
                            },
                        )
                        .into_router()
                        .route_layer(middleware::from_fn(
                            |request: Request<Body>, next: Next<Body>| async {
                                let (mut parts, body) = request.into_parts();
                                let parameters =
                                    Path::<HashMap<String, String>>::from_request_parts(
                                        &mut parts,
                                        &(),
                                    )
                                    .await
                                    .map_err(|rejection| rejection.into_response())
                                    .and_then(|parameters| {
                                        parameters
                                            .get("computer_system_id")
                                            .ok_or(
                                                (
                                                    StatusCode::BAD_REQUEST,
                                                    Json("Missing 'computer_system_id' parameter"),
                                                )
                                                    .into_response(),
                                            )
                                            .and_then(|id| {
                                                u32::from_str_radix(id, 10).map_err(|error| {
                                                    (
                                                        StatusCode::BAD_REQUEST,
                                                        Json(error.to_string()),
                                                    )
                                                        .into_response()
                                                })
                                            })
                                    });
                                let id = match parameters {
                                    Ok(value) => value,
                                    Err(rejection) => return rejection,
                                };

                                let mut request = Request::<Body>::from_parts(parts, body);
                                request.extensions_mut().insert(id);
                                let response = next.run(request).await;
                                response
                            },
                        )),
                )
                .into_router(),
        )
        .layer(TraceLayer::new_for_http());

    redfish_service::serve(config.server, app).await
}
