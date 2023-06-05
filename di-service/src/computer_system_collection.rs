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

use std::{collections::HashMap, convert::Infallible, future::Future, pin::Pin};

use axum::{
    body::Body,
    extract::{FromRequestParts, Path},
    handler::Handler,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{MethodRouter, Route},
    Json, Router,
};
use redfish_codegen::registries::base::v1_15_0::Base;
use seuss::redfish_error;
use tower::Service;
use tracing::{event, Level};

pub fn redfish_map_err<E>(error: E) -> Response
where
    E: std::fmt::Display,
{
    event!(Level::ERROR, "{}", &error);
    (
        StatusCode::BAD_REQUEST,
        Json(redfish_error::one_message(Base::InternalError.into())),
    ).into_response()
}

pub struct ResourceLocator<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    parameter: String,
    handler: H,
}

impl<H, F, R> Clone for ResourceLocator<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            parameter: self.parameter.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl<H, F, R> ResourceLocator<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    pub fn new(parameter: String, handler: H) -> Self {
        Self { parameter, handler }
    }
}

impl<H, F, R> tower::Layer<Route> for ResourceLocator<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    type Service = ResourceLocatorService<H, F, R>;

    fn layer(&self, inner: Route) -> Self::Service {
        ResourceLocatorService::<H, F, R> {
            inner,
            handler: self.handler.clone(),
            parameter: self.parameter.clone(),
        }
    }
}

pub struct ResourceLocatorService<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    inner: Route,
    handler: H,
    parameter: String,
}

impl<H, F, R> Clone for ResourceLocatorService<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            handler: self.handler.clone(),
            parameter: self.parameter.clone(),
        }
    }
}

impl<H, F, R> tower::Service<Request<Body>> for ResourceLocatorService<H, F, R>
where
    H: Service<String, Response = R, Error = Response, Future = F> + Send + Sync + Clone + 'static,
    F: Future<Output = Result<R, Response>> + Send + Sync,
    R: Send + Sync + 'static,
{
    type Response = Response;

    type Error = Infallible;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let parameter = self.parameter.clone();
        let mut handler = self.handler.clone();
        let handler = async move {
            let (mut parts, body) = request.into_parts();
            let parameters = Path::<HashMap<String, String>>::from_request_parts(&mut parts, &())
                .await
                .map_err(|rejection| rejection.into_response())
                .and_then(|parameters| {
                    parameters
                        .get(&parameter)
                        .ok_or(
                            (
                                StatusCode::BAD_REQUEST,
                                Json("Missing '".to_string() + &parameter + "' parameter"),
                            )
                                .into_response(),
                        )
                        .map(|parameter| parameter.clone())
                });
            let id = match parameters {
                Ok(value) => match handler.call(value.clone()).await {
                    Ok(value) => value,
                    Err(rejection) => return Ok::<_, Infallible>(rejection),
                },
                Err(rejection) => return Ok::<_, Infallible>(rejection),
            };

            let mut request = Request::<Body>::from_parts(parts, body);
            request.extensions_mut().insert(id);
            let response = inner.call(request).await;
            response
        };
        Box::pin(handler)
    }
}

#[derive(Default)]
pub struct ComputerSystem(MethodRouter);

impl ComputerSystem {
    pub fn put<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, (), Body>,
        T: 'static,
    {
        Self(self.0.put(handler))
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
    pub fn get<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, (), Body>,
        T: 'static,
    {
        let Self {
            collection,
            systems,
        } = self;
        Self {
            collection: collection.get(handler),
            systems,
        }
    }

    pub fn post<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, (), Body>,
        T: 'static,
    {
        let Self {
            collection,
            systems,
        } = self;
        Self {
            collection: collection.post(handler),
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
