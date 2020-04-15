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
    CountOriginPackagesCallTime,
    GetAllLatestCallTime,
    GetAllPackageCallTime,
    GetGroupPackageCallTime,
    GetLatestChannelPackageCallTime,
    GetLatestPackageCallTime,
    GetPackageCallTime,
    GetWithoutTargetPackageCallTime,
    ListAllChannelPackagesCallTime,
    ListChannelPackagesCallTime,
    ListDistinctForOriginPackageCallTime,
    ListDistinctPackageCallTime,
    ListPackageChannelsPackageCallTime,
    ListPackagePlatformsCallTime,
    ListPackageVersionsPackageCallTime,
    ListPackagesCallTime,
    SearchPackagesCallTime,
    SearchDistinctPackagesCallTime,
}

impl metrics::HistogramMetric for Histogram {}

impl metrics::Metric for Histogram {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Histogram::DbCallTime => "db-call.call-time".into(),
            Histogram::CountOriginPackagesCallTime => {
                "db-call.count-origin-packages-call-time".into()
            }
            Histogram::GetAllLatestCallTime => "db-call.all-latest-call-time".into(),
            Histogram::GetAllPackageCallTime => "db-call.get-all-package-call-time".into(),
            Histogram::GetGroupPackageCallTime => "db-call.get-group-package-call-time".into(),
            Histogram::GetLatestChannelPackageCallTime => {
                "db-call.latest-channel-pkg-call-time".into()
            }
            Histogram::GetLatestPackageCallTime => "db-call.latest-pkg-call-time".into(),
            Histogram::GetPackageCallTime => "db-call.get-pkg-call-time".into(),
            Histogram::GetWithoutTargetPackageCallTime => {
                "db-call.get-without-target-pkg-call-time".into()
            }
            Histogram::ListAllChannelPackagesCallTime => {
                "db-call.list-all-channel-pkgs-call-time".into()
            }
            Histogram::ListChannelPackagesCallTime => "db-call.list-channel-pkgs-call-time".into(),
            Histogram::ListDistinctPackageCallTime => "db-call.list-distinct-pkgs-call-time".into(),
            Histogram::ListDistinctForOriginPackageCallTime => {
                "db-call.list-distinct-for-origin-pkgs-call-time".into()
            }
            Histogram::ListPackageChannelsPackageCallTime => {
                "db-call.list-package-channels-call-time".into()
            }
            Histogram::ListPackagesCallTime => "db-call.list-packages-call-time".into(),
            Histogram::ListPackageVersionsPackageCallTime => {
                "db-call.list-package-versions-call-time".into()
            }
            Histogram::ListPackagePlatformsCallTime => {
                "db-call.list-package-versions-call-time".into()
            }
            Histogram::SearchPackagesCallTime => "db-call.search-packages-call-time".into(),
            Histogram::SearchDistinctPackagesCallTime => {
                "db-call.search-distinct-packages-call-time".into()
            }
        }
    }
}
