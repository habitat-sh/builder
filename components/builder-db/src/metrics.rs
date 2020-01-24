// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

//! Centralized definition of all Builder API metrics that we
//! wish to track.

use crate::bldr_core::metrics;
use std::borrow::Cow;

pub enum Counter {
    DBCall,
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::DBCall => "db-call".into(),
        }
    }
}

pub enum Histogram {
    DbCallTime,
    GetAllLatestCallTime,
    GetLatestChannelPackageCallTime,
    GetLatestPackageCallTime,
    ListAllChannelPackagesCallTime,
}

impl metrics::HistogramMetric for Histogram {}

impl metrics::Metric for Histogram {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Histogram::DbCallTime => "db-call.call-time".into(),
            Histogram::GetAllLatestCallTime => "db-call.all-latest-call-time".into(),
            Histogram::GetLatestChannelPackageCallTime => {
                "db-call.latest-channel-pkg-call-time".into()
            }
            Histogram::GetLatestPackageCallTime => "db-call.latest-pkg-call-time".into(),
            Histogram::ListAllChannelPackagesCallTime => {
                "db-call.list-all-channel-pkgs-call-time".into()
            }
        }
    }
}
