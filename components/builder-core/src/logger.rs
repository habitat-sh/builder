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

use chrono::prelude::*;
use std::{fs::{File,
               OpenOptions},
          io::Write,
          path::Path};

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn init<T, U>(log_path: T, filename: U) -> Self
        where T: AsRef<Path>,
              U: AsRef<Path>
    {
        Logger { file: OpenOptions::new().append(true)
                                         .create(true)
                                         .open(log_path.as_ref().join(filename))
                                         .expect("Failed to initialize log file"), }
    }

    pub fn log(&mut self, msg: &str) {
        let dt: DateTime<Utc> = Utc::now();
        let fmt_msg = format!("{},{}\n", dt.format("%Y-%m-%d %H:%M:%S"), msg);

        self.file
            .write_all(fmt_msg.as_bytes())
            .unwrap_or_else(|_| panic!("Logger unable to write to {:?}", self.file));
    }
}

impl Drop for Logger {
    fn drop(&mut self) { self.file.sync_all().expect("Unable to sync log file"); }
}
