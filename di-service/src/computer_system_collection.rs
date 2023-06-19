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

use std::marker::PhantomData;

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

use crate::PrivilegeTemplate;

pub struct DefaultPrivileges;
impl PrivilegeTemplate for DefaultPrivileges {
    type Get = Login;
    type Post = ConfigureComponents;
    type Put = ConfigureComponents;
    type Patch = ConfigureComponents;
    type Delete = ConfigureComponents;
    type Head = Login;
}

pub struct ComputerSystemCollection<S, P>
where
    S: Clone,
{
    router: MethodRouter<S>,
    systems: Option<Router<S>>,
    marker: PhantomData<fn() -> P>,
}

impl<S> Default for ComputerSystemCollection<S, DefaultPrivileges>
where
    S: Clone,
{
    fn default() -> Self {
        Self {
            router: Default::default(),
            systems: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<S, P> ComputerSystemCollection<S, P>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
    P: PrivilegeTemplate + 'static,
    <P as PrivilegeTemplate>::Get: Send,
    <P as PrivilegeTemplate>::Post: Send,
{
    pub fn get<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        self.router = self.router.get(
            |auth: RedfishAuth<P::Get>, State(state): State<S>, mut request: Request<Body>| async {
                request.extensions_mut().insert(auth.user);
                handler.call(request, state).await
            },
        );
        self
    }

    pub fn post<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        self.router = self.router.post(
            |auth: RedfishAuth<P::Post>, State(state): State<S>, mut request: Request<Body>| async {
                request.extensions_mut().insert(auth.user);
                handler.call(request, state).await
            },
        );
        self
    }

    pub fn systems(mut self, systems: Router<S>) -> Self {
        self.systems = Some(systems);
        self
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            router, systems, ..
        } = self;
        let result = Router::default();
        let result = match systems {
            Some(systems) => result.nest("/:computer_system_id", systems),
            None => result,
        };
        result.route(
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
