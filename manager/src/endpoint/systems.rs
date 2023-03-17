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

use redfish_codegen::api::v1::{computer_system_detail, systems};
use redfish_codegen::models::{
    computer_system::v1_20_0::ComputerSystem, computer_system_collection::ComputerSystemCollection,
    odata_v4, resource,
};
use redfish_codegen::registries::base::v1_15_0::Base;
use seuss::redfish_error;

#[derive(Clone, Default)]
pub struct DummySystem {
    pub odata_id: odata_v4::Id,
    pub name: resource::Name,
}

impl Into<ComputerSystem> for DummySystem {
    fn into(self) -> ComputerSystem {
        let DummySystem { name, odata_id } = self;
        ComputerSystem {
            odata_id,
            name,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default)]
pub struct Systems {
    odata_id: odata_v4::Id,
    systems: Vec<DummySystem>,
}

impl Systems {
    pub fn new(odata_id: odata_v4::Id, systems: Vec<DummySystem>) -> Self {
        Systems { systems, odata_id }
    }
}

impl systems::Systems for Systems {
    fn get(&self) -> systems::SystemsGetResponse {
        systems::SystemsGetResponse::Ok(ComputerSystemCollection {
            odata_id: self.odata_id.clone(),
            ..Default::default()
        })
    }

    fn post(&mut self, _body: ComputerSystem) -> systems::SystemsPostResponse {
        systems::SystemsPostResponse::Default(redfish_error::one_message(
            Base::QueryNotSupportedOnResource.into(),
        ))
    }
}

impl computer_system_detail::ComputerSystemDetail for Systems {
    fn get(&self, id: String) -> computer_system_detail::ComputerSystemDetailGetResponse {
        match self.systems.iter().find(|system| id == system.name.0) {
            Some(system) => {
                computer_system_detail::ComputerSystemDetailGetResponse::Ok(system.clone().into())
            }
            None => computer_system_detail::ComputerSystemDetailGetResponse::Default(
                redfish_error::one_message(Base::ResourceNotFound("type".to_string(), id).into()),
            ),
        }
    }

    fn put(
        &mut self,
        _id: String,
        _body: ComputerSystem,
    ) -> computer_system_detail::ComputerSystemDetailPutResponse {
        todo!()
    }

    fn delete(
        &mut self,
        _id: String,
    ) -> computer_system_detail::ComputerSystemDetailDeleteResponse {
        todo!()
    }

    fn patch(
        &mut self,
        _id: String,
        _body: serde_json::Value,
    ) -> computer_system_detail::ComputerSystemDetailPatchResponse {
        todo!()
    }
}
