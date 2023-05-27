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

use axum::{Router, Json, http::StatusCode};
use clap::Parser;
use seuss::auth::Role;
use redfish_codegen::models::{computer_system_collection::ComputerSystemCollection as Model, redfish};

mod computer_system_collection;

use computer_system_collection::{ComputerSystemCollection, ComputerSystem};
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
        .nest("/redfish/v1/Systems", ComputerSystemCollection::default()
            .read(|| async {
                let model = Model::default();
                Json(model)
            })
            .create(|Json(model): Json<Model>| async {
                event!(Level::INFO, "{}", &serde_json::to_string(&model).map_err(redfish_map_err)?);
                Ok::<_, (StatusCode, Json<redfish::Error>)>(Json(model))
            })
            .systems(ComputerSystem::default())
            .into()
        )
        .layer(TraceLayer::new_for_http());
    
    redfish_service::serve(config.server, app).await
}
