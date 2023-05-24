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

use axum::{Router, routing::MethodRouter, http::StatusCode, Json};
use redfish_codegen::{models::computer_system_collection::ComputerSystemCollection as Model, registries::base::v1_15_0::Base};
use seuss::redfish_error;

pub struct QueryResponse<T> {
    status: StatusCode,
    value: T,
}

impl<T> From<T> for QueryResponse<T> {
    fn from(value: T) -> Self {
        Self {
            status: StatusCode::OK,
            value,
        }
    }
}

#[derive(Default)]
pub struct ComputerSystemCollection(MethodRouter);

impl ComputerSystemCollection {
    pub fn read<Fn, Fut>(self, handler: Fn) -> Self
    where Fn: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = QueryResponse<Model>> + Send,
    {
        Self(self.0.get(|| async move {
            let handler = handler.clone();
            let response = handler().await;
            (response.status, Json(response.value))
        }))
    }
}

impl Into<Router> for ComputerSystemCollection {
    fn into(self) -> Router {
        Router::new()
            .route("/", self.0.fallback(|| async {
                (StatusCode::METHOD_NOT_ALLOWED, Json(redfish_error::one_message(Base::OperationNotAllowed.into())))
            }))
    }
}