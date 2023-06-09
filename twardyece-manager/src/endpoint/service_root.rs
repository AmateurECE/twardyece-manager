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

use redfish_codegen::api::v1;
use redfish_codegen::models::service_root::v1_15_0::Links;
use redfish_codegen::models::{odata_v4, resource, service_root};

#[derive(Clone, Default)]
pub struct ServiceRoot {
    name: resource::Name,
    id: resource::Id,
    odata_id: odata_v4::Id,
    systems: Option<odata_v4::IdRef>,
    session_service: Option<odata_v4::IdRef>,
    sessions_link: odata_v4::IdRef,
}

impl ServiceRoot {
    pub fn new(name: resource::Name, id: resource::Id) -> Self {
        Self {
            name,
            id,
            odata_id: odata_v4::Id(String::default()),
            ..Default::default()
        }
    }

    pub fn enable_systems(mut self) -> Self {
        self.systems = Some(odata_v4::IdRef {
            odata_id: Some(odata_v4::Id("/redfish/v1/Systems".to_string())),
        });
        self
    }

    pub fn enable_sessions(mut self, session_collection_id: odata_v4::Id) -> Self {
        self.session_service = Some(odata_v4::IdRef {
            odata_id: Some(odata_v4::Id("/redfish/v1/SessionService".to_string())),
        });
        self.sessions_link = odata_v4::IdRef {
            odata_id: Some(session_collection_id),
        };
        self
    }
}

impl v1::ServiceRoot for ServiceRoot {
    fn get(&self) -> v1::ServiceRootGetResponse {
        let ServiceRoot {
            name,
            id,
            odata_id,
            systems,
            session_service,
            sessions_link,
        } = self.clone();
        v1::ServiceRootGetResponse::Ok(service_root::v1_15_0::ServiceRoot {
            name,
            id,
            odata_id,
            systems,
            session_service,
            links: Links {
                sessions: sessions_link,
                ..Default::default()
            },
            ..Default::default()
        })
    }
}
