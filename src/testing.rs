// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io;

use crate::type_config::KVChange;
use crate::type_config::KeyOf;
use crate::type_config::TypeConfig;
use crate::type_config::ValueOf;

// Only Debug is actually needed for the test framework
#[derive(Debug, Copy, Clone)]
pub(crate) struct UTTypes {}

impl TypeConfig for UTTypes {
    type Key = String;
    type Value = String;
    type Response = (String, Option<String>, Option<String>);
    type Error = io::Error;

    fn new_initialize_response(key: KeyOf<Self>, value: ValueOf<Self>) -> Self::Response {
        (key, None, Some(value))
    }

    fn new_change_response(change: KVChange<Self>) -> Self::Response {
        change
    }

    fn data_error(error: io::Error) -> Self::Error {
        error
    }

    fn update_watcher_metrics(_delta: i64) {}
}
