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
    GetPackage,
    GetChannelPackage,
    GitHubEvent,
    RouteMessage,
    SearchPackages,
    UploadRequests,
    SingleUploadRequests,
    MultipartUploadRequests,
    DownloadRequests,
    UploadFailures,
    AtomicChannelRequests,
    MemcacheSessionHit,
    MemcacheSessionMiss,
    MemcachePackageHit,
    MemcachePackageMiss,
    MemcachePackage404,
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::GetPackage => "get-package".into(),
            Counter::GetChannelPackage => "get-channel-package".into(),
            Counter::GitHubEvent => "github.event".into(),
            Counter::RouteMessage => "route-message".into(),
            Counter::SearchPackages => "search-packages".into(),
            Counter::UploadRequests => "upload-packages".into(),
            Counter::SingleUploadRequests => "upload-single".into(),
            Counter::MultipartUploadRequests => "upload-multi".into(),
            Counter::DownloadRequests => "download-packages".into(),
            Counter::UploadFailures => "upload-failures".into(),
            Counter::AtomicChannelRequests => "channel-to-channel".into(),
            Counter::MemcacheSessionHit => "memcache-session.hit".into(),
            Counter::MemcacheSessionMiss => "memcache-session.miss".into(),
            Counter::MemcachePackageHit => "memcache-package.hit".into(),
            Counter::MemcachePackageMiss => "memcache-package.miss".into(),
            Counter::MemcachePackage404 => "memcache-package.404".into(),
        }
    }
}

pub enum Histogram {
    MemcacheCallTime,
}

impl metrics::HistogramMetric for Histogram {}

impl metrics::Metric for Histogram {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Histogram::MemcacheCallTime => "memcache.call-time".into(),
        }
    }
}
