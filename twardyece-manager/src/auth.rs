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

use redfish_codegen::models::redfish;
use seuss::auth::{AuthenticatedUser, BasicAuthentication, Role};

#[derive(Clone)]
pub struct ExampleBasicAuthenticator;

impl BasicAuthentication for ExampleBasicAuthenticator {
    fn authenticate(
        &self,
        username: String,
        _password: String,
    ) -> Result<AuthenticatedUser, redfish::Error> {
        Ok(AuthenticatedUser {
            username: username.to_string(),
            role: Role::Administrator,
        })
    }
}
