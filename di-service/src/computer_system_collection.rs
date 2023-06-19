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
