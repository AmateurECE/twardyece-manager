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

use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::MethodRouter,
    Json, Router,
};
use redfish_codegen::{models::redfish, registries::base::v1_15_0::Base};
use seuss::redfish_error;
use tracing::{event, Level};

pub fn redfish_map_err<E>(error: E) -> (StatusCode, Json<redfish::Error>)
where
    E: std::fmt::Display,
{
    event!(Level::ERROR, "{}", &error);
    (
        StatusCode::BAD_REQUEST,
        Json(redfish_error::one_message(Base::InternalError.into())),
    )
}

#[derive(Default)]
pub struct ComputerSystem(MethodRouter);

impl ComputerSystem {
    pub fn replace<Fn, Fut, P, B, Res>(self, handler: Fn) -> Self
    where
        Fn: FnOnce(P, B) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = Res> + Send,
        P: FromRequestParts<()> + Send,
        B: FromRequest<(), Body> + Send,
        Res: IntoResponse,
    {
        Self(self.0.put(|request: Request<Body>| async move {
            let handler = handler.clone();
            let (mut parts, body) = request.into_parts();
            let param = match P::from_request_parts(&mut parts, &()).await {
                Ok(value) => value,
                Err(rejection) => return rejection.into_response(),
            };
            let request = Request::from_parts(parts, body);
            let body = match B::from_request(request, &()).await {
                Ok(value) => value,
                Err(rejection) => return rejection.into_response(),
            };
            handler(param, body).await.into_response()
        }))
    }

    pub fn into_router(self) -> Router {
        Router::new().route("/", self.0)
    }
}

#[derive(Default)]
pub struct ComputerSystemCollection {
    collection: MethodRouter,
    systems: Option<Router>,
}

impl ComputerSystemCollection {
    pub fn read<Fn, Fut, Res>(self, handler: Fn) -> Self
    where
        Fn: FnOnce() -> Fut + Clone + Send + 'static,
        Fut: Future<Output = Res> + Send,
        Res: IntoResponse,
    {
        let Self {
            collection,
            systems,
        } = self;
        Self {
            collection: collection.get(|| async move {
                let handler = handler.clone();
                handler().await
            }),
            systems,
        }
    }

    pub fn create<Fn, Fut, B, Res>(self, handler: Fn) -> Self
    where
        Fn: FnOnce(B) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = Res> + Send,
        B: FromRequest<(), Body> + Send,
        Res: IntoResponse,
    {
        let Self {
            collection,
            systems,
        } = self;
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

    pub fn systems(self, systems: Router) -> Self {
        Self {
            collection: self.collection,
            systems: Some(systems),
        }
    }

    pub fn into_router(self) -> Router {
        let Self {
            collection,
            systems,
        } = self;
        systems
            .map_or(Router::default(), |systems: Router| {
                Router::new().nest("/:computer_system_id", systems)
            })
            .route(
                "/",
                collection.fallback(|| async {
                    (
                        StatusCode::METHOD_NOT_ALLOWED,
                        Json(redfish_error::one_message(Base::OperationNotAllowed.into())),
                    )
                }),
            )
    }
}
