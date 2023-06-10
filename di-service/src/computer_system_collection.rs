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
    collections::HashMap, convert::Infallible, future::Future, marker::PhantomData, pin::Pin,
    str::FromStr,
};

use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Path, State},
    handler::Handler,
    http::{request::Parts, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{MethodRouter, Route},
    Json, Router,
};
use redfish_codegen::registries::base::v1_15_0::Base;
use seuss::{
    auth::{AuthenticateRequest, ConfigureComponents, Login},
    extract::RedfishAuth,
    redfish_error,
};
use tracing::{event, Level};

pub fn redfish_map_err<E>(error: E) -> Response
where
    E: std::fmt::Display,
{
    event!(Level::ERROR, "{}", &error);
    redfish_map_err_no_log(error)
}

pub fn redfish_map_err_no_log<E>(_: E) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(redfish_error::one_message(Base::InternalError.into())),
    )
        .into_response()
}

#[derive(Clone, Default)]
pub struct NoAuth;

impl AuthenticateRequest for NoAuth {
    fn authenticate_request(
        &self,
        _parts: &mut Parts,
    ) -> Result<Option<seuss::auth::AuthenticatedUser>, Response> {
        Ok(None)
    }

    fn challenge(&self) -> Vec<&'static str> {
        // Should never be called, because authenticate_request always returns Ok
        unimplemented!()
    }
}

impl<'a> AsRef<dyn AuthenticateRequest + 'a> for NoAuth {
    fn as_ref(&self) -> &(dyn AuthenticateRequest + 'a) {
        self
    }
}

async fn get_request_parameter<T>(
    mut parts: &mut Parts,
    parameter_name: &String,
) -> Result<T, Response>
where
    T: FromStr,
{
    Path::<HashMap<String, String>>::from_request_parts(&mut parts, &())
        .await
        .map_err(|rejection| rejection.into_response())
        .and_then(|parameters| {
            parameters
                .get(parameter_name)
                .ok_or_else(|| redfish_map_err("Missing '".to_string() + parameter_name + "' parameter"))
                .map(|parameter| parameter.clone())
        })
        .and_then(|value| T::from_str(&value).map_err(redfish_map_err_no_log))
}

#[derive(Clone)]
pub struct FunctionResourceHandler<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

#[async_trait]
pub trait ResourceHandler {
    async fn call(
        self,
        request: Request<Body>,
        parameter_name: String,
    ) -> Result<Request<Body>, Response>;
}

#[async_trait]
impl<T1, T2, Fn, Fut, R> ResourceHandler for FunctionResourceHandler<(T1, T2), Fn>
where
    T1: FromRequestParts<()> + Send,
    T2: FromStr + Send,
    Fn: FnOnce(T1, T2) -> Fut + Send,
    Fut: Future<Output = Result<R, Response>> + Send,
    R: Send + Sync + 'static,
{
    async fn call(
        self,
        request: Request<Body>,
        parameter_name: String,
    ) -> Result<Request<Body>, Response> {
        let (mut parts, body) = request.into_parts();
        let extractor = T1::from_request_parts(&mut parts, &())
            .await
            .map_err(|rejection| rejection.into_response())?;
        let parameter = get_request_parameter::<T2>(&mut parts, &parameter_name)
            .await
            .and_then(|value| Ok((self.f)(extractor, value)))?
            .await?;

        let mut request = Request::<Body>::from_parts(parts, body);
        request.extensions_mut().insert(parameter);
        Ok(request)
    }
}

#[async_trait]
impl<T, Fn, Fut, R> ResourceHandler for FunctionResourceHandler<(T,), Fn>
where
    T: FromStr + Send,
    Fn: FnOnce(T) -> Fut + Send,
    Fut: Future<Output = Result<R, Response>> + Send,
    R: Send + Sync + 'static,
{
    async fn call(
        self,
        request: Request<Body>,
        parameter_name: String,
    ) -> Result<Request<Body>, Response> {
        let (mut parts, body) = request.into_parts();
        let parameter = get_request_parameter(&mut parts, &parameter_name)
            .await
            .and_then(|value| Ok((self.f)(value)))?
            .await?;

        let mut request = Request::<Body>::from_parts(parts, body);
        request.extensions_mut().insert(parameter);
        Ok(request)
    }
}

pub trait IntoResourceHandler<Input> {
    type ResourceHandler;
    fn into_resource_handler(self) -> Self::ResourceHandler;
}

impl<T1, T2, F, R> IntoResourceHandler<(T1, T2)> for F
where
    T1: FromRequestParts<()>,
    T2: FromStr,
    F: FnOnce(T1, T2) -> R,
{
    type ResourceHandler = FunctionResourceHandler<(T1, T2), F>;

    fn into_resource_handler(self) -> Self::ResourceHandler {
        Self::ResourceHandler {
            f: self,
            marker: PhantomData::default(),
        }
    }
}

impl<T, F, R> IntoResourceHandler<(T,)> for F
where
    T: FromStr,
    F: FnOnce(T) -> R,
{
    type ResourceHandler = FunctionResourceHandler<(T,), F>;

    fn into_resource_handler(self) -> Self::ResourceHandler {
        Self::ResourceHandler {
            f: self,
            marker: PhantomData::default(),
        }
    }
}

#[derive(Clone)]
pub struct ResourceLocator<R>
where
    R: ResourceHandler + Clone,
{
    parameter_name: String,
    handler: R,
}

impl<R> ResourceLocator<R>
where
    R: ResourceHandler + Clone,
{
    pub fn new<I>(
        parameter_name: String,
        handler: impl IntoResourceHandler<I, ResourceHandler = R>,
    ) -> Self {
        Self {
            parameter_name,
            handler: handler.into_resource_handler(),
        }
    }
}

impl<R> tower::Layer<Route> for ResourceLocator<R>
where
    R: ResourceHandler + Clone,
{
    type Service = ResourceLocatorService<R>;

    fn layer(&self, inner: Route) -> Self::Service {
        ResourceLocatorService {
            inner,
            handler: self.handler.clone(),
            parameter_name: self.parameter_name.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ResourceLocatorService<R>
where
    R: ResourceHandler,
{
    inner: Route,
    handler: R,
    parameter_name: String,
}

impl<R> tower::Service<Request<Body>> for ResourceLocatorService<R>
where
    R: ResourceHandler + Send + Sync + Clone + 'static,
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
        let parameter_name = self.parameter_name.clone();
        let handler = self.handler.clone();
        let handler = async move {
            let request = match handler.call(request, parameter_name).await {
                Ok(value) => value,
                Err(rejection) => return Ok::<_, Infallible>(rejection),
            };
            let response = inner.call(request).await;
            response
        };
        Box::pin(handler)
    }
}

#[derive(Default)]
pub struct CertificateCollection<S>
where
    S: Clone,
{
    router: MethodRouter<S>,
    certificates: Option<Router<S>>,
}

impl<S> CertificateCollection<S>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
{
    pub fn get<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        let Self {
            router,
            certificates,
        } = self;
        Self {
            router: router.get(
                |auth: RedfishAuth<ConfigureComponents>,
                 State(state): State<S>,
                 mut request: Request<Body>| async {
                    request.extensions_mut().insert(auth.user);
                    handler.call(request, state).await
                },
            ),
            certificates,
        }
    }

    pub fn certificates(self, certificates: Router<S>) -> Self {
        let Self { router, .. } = self;
        Self {
            router,
            certificates: Some(certificates),
        }
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            router,
            certificates,
        } = self;
        certificates
            .map_or(Router::default(), |certificates| {
                Router::new().nest("/:certificate_id", certificates)
            })
            .route(
                "/",
                router.fallback(|| async {
                    (
                        StatusCode::METHOD_NOT_ALLOWED,
                        Json(redfish_error::one_message(Base::OperationNotAllowed.into())),
                    )
                }),
            )
    }
}

#[derive(Default)]
pub struct Certificate<S>(MethodRouter<S>)
where
    S: Clone;

impl<S> Certificate<S>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
{
    pub fn get<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        // The privilege "ConfigureManager" is the default required for the
        // Certificate component, but Redfish Privilege Mapping 1.3.1 specifies
        // a subordinate override for the component ComputerSystem.
        Self(self.0.get(
            |auth: RedfishAuth<ConfigureComponents>,
             State(state): State<S>,
             mut request: Request<Body>| async {
                request.extensions_mut().insert(auth.user);
                handler.call(request, state).await
            },
        ))
    }

    pub fn into_router(self) -> Router<S> {
        Router::new().route("/", self.0)
    }
}

#[derive(Default)]
pub struct ComputerSystem<S>
where
    S: Clone,
{
    router: MethodRouter<S>,
    certificates: Option<Router<S>>,
}

impl<S> ComputerSystem<S>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
{
    pub fn put<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        let Self {
            router,
            certificates,
        } = self;
        Self {
            router: router.put(
                |auth: RedfishAuth<ConfigureComponents>,
                 State(state): State<S>,
                 mut request: Request<Body>| async {
                    request.extensions_mut().insert(auth.user);
                    handler.call(request, state).await
                },
            ),
            certificates,
        }
    }

    pub fn certificates(self, router: Router<S>) -> Self {
        Self {
            router: self.router,
            certificates: Some(router),
        }
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            router,
            certificates,
        } = self;
        Router::new()
            .route("/", router)
            .nest("/Certificates", certificates.unwrap())
    }
}

#[derive(Default)]
pub struct ComputerSystemCollection<S>
where
    S: Clone,
{
    collection: MethodRouter<S>,
    systems: Option<Router<S>>,
}

impl<S> ComputerSystemCollection<S>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
{
    pub fn get<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        let Self {
            collection,
            systems,
        } = self;
        Self {
            collection:
                collection.get(
                    |auth: RedfishAuth<Login>,
                     State(state): State<S>,
                     mut request: Request<Body>| async {
                        request.extensions_mut().insert(auth.user);
                        handler.call(request, state).await
                    },
                ),
            systems,
        }
    }

    pub fn post<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        let Self {
            collection,
            systems,
        } = self;
        Self {
            collection: collection.post(
                |auth: RedfishAuth<ConfigureComponents>,
                 State(state): State<S>,
                 mut request: Request<Body>| async {
                    request.extensions_mut().insert(auth.user);
                    handler.call(request, state).await
                },
            ),
            systems,
        }
    }

    pub fn systems(self, systems: Router<S>) -> Self {
        Self {
            collection: self.collection,
            systems: Some(systems),
        }
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            collection,
            systems,
        } = self;
        systems
            .map_or(Router::default(), |systems| {
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
