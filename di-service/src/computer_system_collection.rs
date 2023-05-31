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
use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts, Path, State},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::MethodRouter,
    Json, Router,
};
use redfish_codegen::{models::redfish, registries::base::v1_15_0::Base};
use seuss::redfish_error;
use tower::Service;
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

pub struct ComputerSystem<S, I, E, F>(MethodRouter<S>)
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone + 'static,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send;

impl<S, I, E, F> Default for ComputerSystem<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone + 'static,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    fn default() -> Self {
        Self(MethodRouter::<S>::default())
    }
}

impl<S, I, E, F> ComputerSystem<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    pub fn replace<Fn, Fut, B, R>(self, handler: Fn) -> Self
    where
        Fn: FnOnce(I, B) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = R> + Send,
        B: FromRequest<S, Body> + Send,
        R: IntoResponse,
    {
        Self(self.0.put(
            |State(state): State<S>, request: Request<Body>| async move {
                let handler = handler.clone();
                let (mut parts, body) = request.into_parts();
                let mut state: State<S> = match State::from_request_parts(&mut parts, &state).await
                {
                    Ok(value) => value,
                    Err(rejection) => return rejection.into_response(),
                };

                let Path(parameters) =
                    match Path::<HashMap<String, String>>::from_request_parts(&mut parts, &state)
                        .await
                    {
                        Ok(value) => value,
                        Err(rejection) => return rejection.into_response(),
                    };
                let item = match state.call(parameters).await {
                    Ok(value) => value,
                    Err(rejection) => return rejection.into_response(),
                };

                let request = Request::from_parts(parts, body);
                let body = match B::from_request(request, &state).await {
                    Ok(value) => value,
                    Err(rejection) => return rejection.into_response(),
                };
                handler(item, body).await.into_response()
            },
        ))
    }
}

impl<S, I, E, F> Into<Router<S>> for ComputerSystem<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    fn into(self) -> Router<S> {
        Router::new().route("/", self.0)
    }
}

pub struct ComputerSystemCollection<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone + 'static,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    collection: MethodRouter,
    systems: Option<ComputerSystem<S, I, E, F>>,
    locator: Option<S>,
}

impl<S, I, E, F> Default for ComputerSystemCollection<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone + 'static,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    fn default() -> Self {
        Self {
            collection: MethodRouter::default(),
            systems: None,
            locator: None,
        }
    }
}

impl<S, I, E, F> ComputerSystemCollection<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    pub fn read<Fn, Fut, R>(self, handler: Fn) -> Self
    where
        Fn: FnOnce() -> Fut + Clone + Send + 'static,
        Fut: Future<Output = R> + Send,
        R: IntoResponse,
    {
        let Self {
            collection,
            systems,
            locator,
        } = self;
        Self {
            collection: collection.get(|| async move {
                let handler = handler.clone();
                handler().await
            }),
            systems,
            locator,
        }
    }

    pub fn create<Fn, Fut, B, R>(self, handler: Fn) -> Self
    where
        Fn: FnOnce(B) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = R> + Send,
        B: FromRequest<(), Body> + Send,
        R: IntoResponse,
    {
        let Self {
            collection,
            systems,
            locator,
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
            locator,
        }
    }

    pub fn systems(self, systems: ComputerSystem<S, I, E, F>, locator: S) -> Self {
        Self {
            collection: self.collection,
            systems: Some(systems),
            locator: Some(locator),
        }
    }
}

impl<S, I, E, F> Into<Router> for ComputerSystemCollection<S, I, E, F>
where
    S: Service<HashMap<String, String>, Response = I, Error = E, Future = F>
        + Send
        + Sync
        + Clone
        + 'static,
    I: Send + Sync + Clone,
    E: IntoResponse + Send + Sync + Clone,
    F: Future<Output = Result<I, E>> + Send,
{
    fn into(self) -> Router {
        let Self {
            collection,
            systems,
            locator,
        } = self;
        let systems: Router<S> = systems.unwrap().into();
        Router::new()
            .route(
                "/",
                collection.fallback(|| async {
                    (
                        StatusCode::METHOD_NOT_ALLOWED,
                        Json(redfish_error::one_message(Base::OperationNotAllowed.into())),
                    )
                }),
            )
            .nest("/:computer_system", systems.with_state(locator.unwrap()))
    }
}
