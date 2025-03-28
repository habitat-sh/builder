// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use habitat_core::{crypto::keys::{BuilderSecretEncryptionKey, KeyCache, NamedRevision}, Error};

pub const BUILDER_KEY_NAME: &str = "bldr";

pub fn get_latest_builder_key(key_cache: &KeyCache) -> Result<BuilderSecretEncryptionKey, Error> {
    key_cache.latest_builder_key()
}

pub fn get_builder_key_for_revision(key_cache: &KeyCache, named_revision: &NamedRevision) -> Result<BuilderSecretEncryptionKey, Error> {
    key_cache.builder_secret_encryption_key(named_revision)
}
