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

use redfish_codegen::{
    api::v1::session_service::{self, sessions},
    models::{
        odata_v4, resource, session::v1_6_0, session_collection::SessionCollection,
        session_service::v1_1_8,
    },
    registries::base::v1_15_0::Base,
};
use seuss::{auth::AuthenticateRequest, redfish_error};

#[derive(Clone)]
pub struct DisabledSessionService<S>
where
    S: Clone + AuthenticateRequest,
{
    id: resource::Id,
    name: resource::Name,
    odata_id: odata_v4::Id,
    sessions: odata_v4::Id,
    auth_handler: S,
}

impl<S> AsRef<dyn AuthenticateRequest> for DisabledSessionService<S>
where
    S: Clone + AuthenticateRequest + 'static,
{
    fn as_ref(&self) -> &(dyn AuthenticateRequest + 'static) {
        &self.auth_handler
    }
}

impl<S> DisabledSessionService<S>
where
    S: Clone + AuthenticateRequest,
{
    pub fn new(
        odata_id: odata_v4::Id,
        name: resource::Name,
        sessions: odata_v4::Id,
        auth_handler: S,
    ) -> Self {
        DisabledSessionService {
            id: resource::Id("sessions".to_string()),
            name,
            odata_id,
            sessions,
            auth_handler,
        }
    }
}

impl<S> session_service::SessionService for DisabledSessionService<S>
where
    S: Clone + AuthenticateRequest,
{
    fn get(&self) -> session_service::SessionServiceGetResponse {
        session_service::SessionServiceGetResponse::Ok(v1_1_8::SessionService {
            id: self.id.clone(),
            name: self.name.clone(),
            odata_id: self.odata_id.clone(),
            service_enabled: Some(false),
            sessions: Some(odata_v4::IdRef {
                odata_id: Some(self.sessions.clone()),
            }),
            ..Default::default()
        })
    }

    fn put(&mut self, _body: v1_1_8::SessionService) -> session_service::SessionServicePutResponse {
        session_service::SessionServicePutResponse::Default(redfish_error::one_message(
            Base::QueryNotSupportedOnResource.into(),
        ))
    }

    fn patch(&mut self, _body: serde_json::Value) -> session_service::SessionServicePatchResponse {
        session_service::SessionServicePatchResponse::Default(redfish_error::one_message(
            Base::QueryNotSupportedOnResource.into(),
        ))
    }
}

#[derive(Clone)]
pub struct EmptySessionCollection<S> {
    odata_id: odata_v4::Id,
    name: resource::Name,
    auth_handler: S,
}

impl<S> AsRef<dyn AuthenticateRequest> for EmptySessionCollection<S>
where
    S: Clone + AuthenticateRequest + 'static,
{
    fn as_ref(&self) -> &(dyn AuthenticateRequest + 'static) {
        &self.auth_handler
    }
}

impl<S> EmptySessionCollection<S>
where
    S: Clone + AuthenticateRequest,
{
    pub fn new(odata_id: odata_v4::Id, name: resource::Name, auth_handler: S) -> Self {
        Self {
            odata_id,
            name,
            auth_handler,
        }
    }
}

impl<S> sessions::Sessions for EmptySessionCollection<S>
where
    S: Clone + AuthenticateRequest,
{
    fn get(&self) -> sessions::SessionsGetResponse {
        sessions::SessionsGetResponse::Ok(SessionCollection {
            name: self.name.clone(),
            odata_id: self.odata_id.clone(),
            ..Default::default()
        })
    }

    fn post(&mut self, _: v1_6_0::Session) -> sessions::SessionsPostResponse {
        sessions::SessionsPostResponse::Default(redfish_error::one_message(
            Base::OperationNotAllowed.into(),
        ))
    }
}
