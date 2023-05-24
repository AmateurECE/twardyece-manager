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

use axum::{Router, routing::get, http::StatusCode, Json};
use redfish_codegen::models::computer_system_collection::ComputerSystemCollection as Model;

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
pub struct ComputerSystemCollection(Router);

impl ComputerSystemCollection {
    pub fn read<Fn, Fut>(self, handler: Fn) -> Self
    where Fn: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = QueryResponse<Model>> + Send + Sync + 'static,
    {
        Self(self.0.route("/", get(|| async move {
            let handler = handler.clone();
            let response = handler().await;
            (response.status, Json(response.value))
        })))
    }
}

impl Into<Router> for ComputerSystemCollection {
    fn into(self) -> Router {
        self.0
    }
}