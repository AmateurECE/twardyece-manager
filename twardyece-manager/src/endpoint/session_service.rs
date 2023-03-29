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
    api::v1::session_service, models::session_service::v1_1_8, registries::base::v1_15_0::Base,
};
use seuss::redfish_error;

#[derive(Clone)]
pub struct SessionService {}

impl SessionService {
    pub fn new() -> Self {
        SessionService {}
    }
}

impl session_service::SessionService for SessionService {
    fn get(&self) -> session_service::SessionServiceGetResponse {
        session_service::SessionServiceGetResponse::Ok(v1_1_8::SessionService {
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
