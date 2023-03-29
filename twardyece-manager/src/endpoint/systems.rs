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
    computer_system::v1_20_0::{Actions, ComputerSystem, Reset, ResetRequestBody},
    computer_system_collection::ComputerSystemCollection,
    odata_v4, resource,
};
use redfish_codegen::registries::base::v1_15_0::Base;
use seuss::{auth::AuthenticateRequest, redfish_error};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct DummySystem {
    pub odata_id: odata_v4::Id,
    pub name: resource::Name,
    pub power_state: resource::PowerState,
}

impl Into<ComputerSystem> for DummySystem {
    fn into(self) -> ComputerSystem {
        let DummySystem {
            name,
            odata_id,
            power_state,
        } = self;
        let id = resource::Id(name.0.clone());
        ComputerSystem {
            odata_id: odata_id.clone(),
            name,
            id,
            power_state: Some(power_state),
            actions: Some(Actions {
                computer_system_reset: Some(Reset {
                    target: Some(odata_id.0 + "/Actions/ComputerSystem.Reset"),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct Systems<S>
where
    S: Clone + AuthenticateRequest,
{
    odata_id: odata_v4::Id,
    systems: Arc<Mutex<Vec<DummySystem>>>,
    name: resource::Name,
    auth_handler: S,
}

impl<S> Systems<S>
where
    S: Clone + AuthenticateRequest,
{
    pub fn new(
        odata_id: odata_v4::Id,
        name: resource::Name,
        systems: Vec<DummySystem>,
        auth_handler: S,
    ) -> Self {
        Systems {
            odata_id,
            systems: Arc::new(Mutex::new(systems)),
            name,
            auth_handler,
        }
    }
}

impl<S> AsRef<dyn AuthenticateRequest> for Systems<S>
where
    S: Clone + AuthenticateRequest + 'static,
{
    fn as_ref(&self) -> &(dyn AuthenticateRequest + 'static) {
        &self.auth_handler
    }
}

impl<S> systems::Systems for Systems<S>
where
    S: AuthenticateRequest + Clone,
{
    fn get(&self) -> systems::SystemsGetResponse {
        let systems = self.systems.lock().unwrap();
        systems::SystemsGetResponse::Ok(ComputerSystemCollection {
            odata_id: self.odata_id.clone(),
            members: systems
                .iter()
                .map(|system| odata_v4::IdRef {
                    odata_id: Some(odata_v4::Id(system.odata_id.0.clone())),
                })
                .collect(),
            name: self.name.clone(),
            members_odata_count: odata_v4::Count(systems.len().try_into().unwrap()),
            ..Default::default()
        })
    }

    fn post(&mut self, _body: ComputerSystem) -> systems::SystemsPostResponse {
        systems::SystemsPostResponse::Default(redfish_error::one_message(
            Base::QueryNotSupportedOnResource.into(),
        ))
    }
}

impl<S> computer_system_detail::ComputerSystemDetail for Systems<S>
where
    S: Clone + AuthenticateRequest,
{
    fn get(&self, id: String) -> computer_system_detail::ComputerSystemDetailGetResponse {
        match self
            .systems
            .lock()
            .unwrap()
            .iter()
            .find(|system| id == system.name.0)
        {
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

impl<S> computer_system_detail::reset::Reset for Systems<S>
where
    S: AuthenticateRequest + Clone,
{
    fn post(
        &mut self,
        _id: String,
        id: String,
        body: ResetRequestBody,
    ) -> computer_system_detail::reset::ResetPostResponse {
        use computer_system_detail::reset::ResetPostResponse;
        use resource::ResetType::*;
        match self
            .systems
            .lock()
            .unwrap()
            .iter_mut()
            .find(|system| id == system.name.0)
        {
            Some(system) => {
                if body.reset_type.is_none() {
                    let message =
                        Base::ActionParameterMissing("Reset".to_string(), "ResetType".to_string());
                    return ResetPostResponse::Default(redfish_error::one_message(message.into()));
                }
                let reset_type = body.reset_type.unwrap();

                match reset_type {
                    GracefulRestart | ForceRestart | On | ForceOn | PowerCycle => {
                        system.power_state = resource::PowerState::On;
                        ResetPostResponse::Ok(redfish_error::one_message(Base::Success.into()))
                    }
                    ForceOff | GracefulShutdown => {
                        system.power_state = resource::PowerState::Off;
                        ResetPostResponse::Ok(redfish_error::one_message(Base::Success.into()))
                    }
                    Nmi | Suspend | Pause | Resume => {
                        ResetPostResponse::Default(redfish_error::one_message(
                            Base::PropertyNotUpdated("PowerState".to_string()).into(),
                        ))
                    }
                    PushPowerButton => {
                        match system.power_state {
                            resource::PowerState::On | resource::PowerState::PoweringOn => {
                                system.power_state = resource::PowerState::Off
                            }
                            resource::PowerState::Off | resource::PowerState::PoweringOff => {
                                system.power_state = resource::PowerState::On
                            }
                            resource::PowerState::Paused => {
                                return ResetPostResponse::Default(redfish_error::one_message(
                                    Base::PropertyValueError("PowerState".to_string()).into(),
                                ))
                            }
                        };
                        ResetPostResponse::Ok(redfish_error::one_message(Base::Success.into()))
                    }
                }
            }
            None => {
                let message = Base::ActionParameterMissing(
                    "ComputerSystem.Reset".to_string(),
                    "ResetType".to_string(),
                );
                ResetPostResponse::Default(redfish_error::one_message(message.into()))
            }
        }
    }
}
