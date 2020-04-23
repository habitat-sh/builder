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

// Not quite alphabetically ordered
pub enum Histogram {
    DbCallTime,
    ChannelGetLatestPackageCallTime,
    ChannelListAllPackagesCallTime,
    ChannelListPackagesCallTime,
    ChannelListPackagesOriginNameCallTime,
    ChannelListPackagesOriginOnlyCallTime,
    PackageCountOriginPackages,
    PackageGetAllCallTime,
    PackageGetAllLatestCallTime,
    PackageGetCallTime,
    PackageGetGroupCallTime,
    PackageGetLatestCallTime,
    PackageGetWithoutTargetCallTime,
    PackageListCallTime,
    PackageListOriginNameCallTime,
    PackageListOriginOnlyCallTime,
    PackageListDistinctCallTime,
    PackageListDistinctOriginNameCallTime,
    PackageListDistinctOriginOnlyCallTime,
    PackageListDistinctForOriginCallTime,
    PackageListPackageChannelsCallTime,
    PackageListPackagePlatformsCallTime,
    PackageListPackageVersionsCallTime,
    PackageSearchCallTime,
    PackageSearchDistinctCallTime,
}

impl metrics::HistogramMetric for Histogram {}

impl metrics::Metric for Histogram {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Histogram::DbCallTime => "db-call.call-time".into(),

            Histogram::ChannelGetLatestPackageCallTime => {
                "db-call.channel-get-latest-package-call-time".into()
            }
            Histogram::ChannelListAllPackagesCallTime => {
                "db-call.channel-list-all-packages-call-time".into()
            }
            Histogram::ChannelListPackagesCallTime => {
                "db-call.channel-list-packages-call-time".into()
            }
            Histogram::ChannelListPackagesOriginNameCallTime => {
                "db-call.channel-list-packages-origin-name-call-time".into()
            }
            Histogram::ChannelListPackagesOriginOnlyCallTime => {
                "db-call.channel-list-packages-origin-only-call-time".into()
            }

            Histogram::PackageCountOriginPackages => {
                "db-call.package-count-origin-packages-call-time".into()
            }
            Histogram::PackageGetAllCallTime => "db-call.package-get-all-call-time".into(),
            Histogram::PackageGetAllLatestCallTime => {
                "db-call.package-get-all-latest-call-time".into()
            }
            Histogram::PackageGetCallTime => "db-call.package-get-call-time".into(),
            Histogram::PackageGetGroupCallTime => "db-call.package-get-group-call-time".into(),
            Histogram::PackageGetLatestCallTime => "db-call.package-get-latest-call-time".into(),
            Histogram::PackageGetWithoutTargetCallTime => {
                "db-call.package-get-without-target-call-time".into()
            }
            Histogram::PackageListCallTime => "db-call.package-list-call-time".into(),
            Histogram::PackageListOriginNameCallTime => {
                "db-call.package-list-origin-name-call-time".into()
            }
            Histogram::PackageListOriginOnlyCallTime => {
                "db-call.package-list-origin-only-call-time".into()
            }
            Histogram::PackageListDistinctCallTime => {
                "db-call.package-list-distinct-call-time".into()
            }
            Histogram::PackageListDistinctOriginNameCallTime => {
                "db-call.package-list-distinct-origin-name-call-time".into()
            }
            Histogram::PackageListDistinctOriginOnlyCallTime => {
                "db-call.package-list-distinct-origin-only-call-time".into()
            }
            Histogram::PackageListDistinctForOriginCallTime => {
                "db-call.package-list-distinct-for-origin-call-time".into()
            }
            Histogram::PackageListPackageChannelsCallTime => {
                "db-call.list-package-list-package-channels-call-time".into()
            }
            Histogram::PackageListPackagePlatformsCallTime => {
                "db-call.package-list-package-platforms-call-time".into()
            }
            Histogram::PackageListPackageVersionsCallTime => {
                "db-call.package-list-package-versions-call-time".into()
            }
            Histogram::PackageSearchCallTime => "db-call.package-search-call-time".into(),
            Histogram::PackageSearchDistinctCallTime => {
                "db-call.package-search-distinct-call-time".into()
            }
        }
    }
}
