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
    body::Body,
    extract::State,
    handler::Handler,
    http::{Request, StatusCode},
    routing::MethodRouter,
    Json, Router,
};
use redfish_codegen::registries::base::v1_15_0::Base;
use redfish_core::{
    auth::AuthenticateRequest,
    error,
    extract::RedfishAuth,
    privilege::{ConfigureComponents, Login},
};

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
                        Json(error::one_message(Base::OperationNotAllowed.into())),
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
                        Json(error::one_message(Base::OperationNotAllowed.into())),
                    )
                }),
            )
    }
}
