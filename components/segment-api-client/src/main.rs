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

// NOTE: This is only here to allow manual testing of the API client.
use segment_api_client as segment;

use std::{env,
          process::exit};

use crate::segment::{client::SegmentClient,
                     config::SegmentCfg};

fn main() {
    let mut config = SegmentCfg::default();
    match env::args().nth(1) {
        Some(w) => config.write_key = w,
        None => {
            println!("Usage: segment-client WRITE_KEY");
            exit(1);
        }
    }

    let client = SegmentClient::new(config).expect("Valid segment client");
    client.identify("abc123");
    client.track("abc123", "tested tracking");
}
