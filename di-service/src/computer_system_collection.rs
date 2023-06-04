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

use axum::{body::Body, handler::Handler, http::StatusCode, routing::MethodRouter, Json, Router};
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
