// Copyright (c) 2018-2025 Progress Software Corporation and/or its subsidiaries, affiliates or applicable contributors. All Rights Reserved.
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

#[allow(mismatched_lifetime_syntaxes, renamed_and_removed_lints)]
pub mod originsrv;

use crate::error::ProtocolError;

pub fn decode<T>(bytes: &[u8]) -> Result<T, ProtocolError>
    where T: protobuf::Message
{
    protobuf::Message::parse_from_bytes(bytes).map_err(ProtocolError::Decode)
}

pub fn encode<T>(message: &T) -> Result<Vec<u8>, ProtocolError>
    where T: protobuf::Message
{
    message.write_to_bytes().map_err(ProtocolError::Encode)
}
