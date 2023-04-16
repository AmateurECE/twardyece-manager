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

use axum::{
    extract::Host,
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::Redirect,
    Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use clap::Parser;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use redfish_codegen::models::{odata_v4, resource};
use seuss::{
    auth::{pam::LinuxPamAuthenticator, CombinedAuthenticationProxy, Role},
    routing,
    service::{self, session_manager::InMemorySessionManager},
};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;
use std::fs::File;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf};
use tower_http::{trace::TraceLayer, BoxError};

mod auth;
mod endpoint;

#[derive(Copy, Clone, serde::Deserialize)]
struct Ports {
    http: u16,
    https: u16,
}

#[derive(serde::Deserialize)]
struct Configuration {
    #[serde(rename = "role-map")]
    role_map: HashMap<Role, String>,
    address: String,
    ports: Ports,
    #[serde(rename = "certificate-file")]
    certificate_file: String,
    #[serde(rename = "key-file")]
    key_file: String,
}

#[derive(Parser)]
struct Args {
    /// Configuration file
    #[clap(value_parser, short, long)]
    config: String,
}

async fn redirect_http_to_https(address: String, ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr: SocketAddr = (address + ":" + &ports.http.to_string()).parse().unwrap();
    tracing::debug!("http redirect listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(redirect.into_make_service())
        .await
        .unwrap();
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

    let service_document = service::OData::new()
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
        .route("/redfish", service::RedfishVersions::default().into())
        .route(
            "/redfish/v1",
            axum::routing::get(|| async { Redirect::permanent("/redfish/v1/") }),
        )
        .route(
            "/redfish/v1/",
            routing::ServiceRoot::new(service_root).into(),
        )
        .route("/redfish/v1/odata", service_document.into())
        .route(
            "/redfish/v1/$metadata",
            redfish_codegen::routing::Metadata.into(),
        )
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

    let server_handle = Handle::new();
    let signals = Signals::new(&[SIGINT, SIGTERM])?;
    let signals_handle = signals.handle();
    let signal_handler = |mut signals: Signals| async move {
        if let Some(_) = signals.next().await {
            println!("Gracefully shutting down");
            server_handle.shutdown();
        }
    };

    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from(config.certificate_file),
        PathBuf::from(config.key_file),
    )
    .await
    .unwrap();

    let https_address = config.address.clone() + ":" + &config.ports.https.to_string();

    let signals_task = tokio::spawn(signal_handler(signals)).fuse();
    let https_server = axum_server::bind_rustls(https_address.parse().unwrap(), tls_config)
        .serve(app.into_make_service())
        .fuse();
    let http_server = tokio::spawn(redirect_http_to_https(config.address, config.ports)).fuse();

    futures::pin_mut!(signals_task, https_server, http_server);
    futures::select! {
        _ = https_server => {},
        _ = http_server => {},
        _ = signals_task => {},
    };

    signals_handle.close();
    Ok(())
}
