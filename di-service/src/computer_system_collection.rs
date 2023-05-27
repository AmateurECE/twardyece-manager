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

use core::future::Future;

use axum::{Router, routing::MethodRouter, http::{StatusCode, Request}, Json, body::Body, extract::FromRequest, response::IntoResponse};
use redfish_codegen::{models::redfish, registries::base::v1_15_0::Base};
use seuss::redfish_error;
use tracing::{event, Level};

pub fn redfish_map_err<E>(error: E) -> (StatusCode, Json<redfish::Error>)
where E: std::fmt::Display,
{
    event!(Level::ERROR, "{}", &error);
    (StatusCode::BAD_REQUEST, Json(redfish_error::one_message(Base::InternalError.into())))
}

#[derive(Default)]
pub struct ComputerSystem(MethodRouter);

impl Into<Router> for ComputerSystem {
    fn into(self) -> Router {
        Router::new()
            .route("/", self.0)
    }
}

#[derive(Default)]
pub struct ComputerSystemCollection {
    collection: MethodRouter,
    systems: ComputerSystem,
}

impl ComputerSystemCollection {
    pub fn read<Fn, Fut, R>(self, handler: Fn) -> Self
    where Fn: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = R> + Send,
    R: IntoResponse + Send,
    {
        let Self { collection, systems } = self;
        Self {
            collection: collection.get(|| async move {
                let handler = handler.clone();
                handler().await
            }),
            systems,
        }
    }

    pub fn create<Fn, Fut, B, R>(self, handler: Fn) -> Self
    where Fn: FnOnce(B) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = R> + Send,
    B: FromRequest<(), Body> + Send,
    R: IntoResponse + Send,
    {
        let Self { collection, systems } = self;
        Self {
            collection: collection.post(|request: Request<Body>| async move {
                let handler = handler.clone();
                let body = match B::from_request(request, &()).await {
                    Ok(value) => value,
                    Err(rejection) => return rejection.into_response(),
                };
                handler(body).await.into_response()
            }),
            systems,
        }
    }

    pub fn systems(self, systems: ComputerSystem) -> Self {
        Self {
            collection: self.collection,
            systems,
        }
    }
}

impl Into<Router> for ComputerSystemCollection {
    fn into(self) -> Router {
        Router::new()
            .route("/", self.collection.fallback(|| async {
                (StatusCode::METHOD_NOT_ALLOWED, Json(redfish_error::one_message(Base::OperationNotAllowed.into())))
            }))
            .nest("/:computer_system", self.systems.into())
    }
}