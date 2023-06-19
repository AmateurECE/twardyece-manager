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

use axum::{response::Response, Extension, Json, Router};
use certificate::Certificate;
use certificate_collection::CertificateCollection;
use clap::Parser;
use computer_system::{CertificateCollectionPrivileges, CertificatePrivileges, ComputerSystem};
use redfish_codegen::models::{
    certificate_collection::CertificateCollection as CertificateCollectionModel,
    computer_system::v1_20_0::ComputerSystem as System,
    computer_system_collection::ComputerSystemCollection as Model,
};

mod certificate;
mod certificate_collection;
mod computer_system;
mod computer_system_collection;

use computer_system_collection::ComputerSystemCollection;
use redfish_core::privilege::{Role, SatisfiesPrivilege};
use seuss::{auth::NoAuth, error::redfish_map_err, middleware::ResourceLocator};
use tower_http::trace::TraceLayer;
use tracing::{event, Level};

pub trait OperationPrivilegeMapping {
    type Get: SatisfiesPrivilege;
    type Head: SatisfiesPrivilege;
    type Post: SatisfiesPrivilege;
    type Put: SatisfiesPrivilege;
    type Patch: SatisfiesPrivilege;
    type Delete: SatisfiesPrivilege;
}

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
    server: seuss::router::Configuration,
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
                    Ok::<_, Response>(Json(model))
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
                                Ok::<_, Response>(Json(system))
                            },
                        )
                        .certificates(
                            CertificateCollection::<_, CertificateCollectionPrivileges>::new()
                                .get(|| async { Json(CertificateCollectionModel::default()) })
                                .certificates(
                                    Certificate::<_, CertificatePrivileges>::new()
                                        .get(|Extension(system): Extension<u32>, Extension(id): Extension<String>| async move {
                                            event!(Level::INFO, "computer_system_id={}, certificate_id={}", system, id);
                                        })
                                        .into_router()
                                        .route_layer(ResourceLocator::new(
                                            "certificate_id".to_string(),
                                            |Extension(system): Extension<u32>, id: String| async move {
                                                event!(Level::INFO, "in middleware, system is {}", system);
                                                Ok::<_, Response>(id)
                                            }
                                        )),
                                )
                                .into_router(),
                        )
                        .into_router()
                        .route_layer(ResourceLocator::new(
                            "computer_system_id".to_string(),
                            |id: u32| async move { Ok::<_, Response>(id) },
                        )),
                )
                .into_router()
                .with_state(NoAuth),
        )
        .layer(TraceLayer::new_for_http());

    seuss::router::serve(config.server, app).await
}
