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

use std::borrow::Cow;

use crate::bldr_core::metrics;
use crate::hab_core::package::PackageTarget;

pub enum Counter {
    CompletedJobs(PackageTarget),
    FailedJobs(PackageTarget),
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::CompletedJobs(ref t) => format!("jobsrv.completed.{}", t).into(),
            Counter::FailedJobs(ref t) => format!("jobsrv.failed.{}", t).into(),
        }
    }
}

pub enum Gauge {
    WaitingJobs(PackageTarget),
    WorkingJobs(PackageTarget),
    Workers(PackageTarget),
    BusyWorkers(PackageTarget),
    ReadyWorkers(PackageTarget),
}

impl metrics::GaugeMetric for Gauge {}

impl metrics::Metric for Gauge {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Gauge::WaitingJobs(ref t) => format!("jobsrv.waiting.{}", t).into(),
            Gauge::WorkingJobs(ref t) => format!("jobsrv.working.{}", t).into(),
            Gauge::Workers(ref t) => format!("jobsrv.workers.{}", t).into(),
            Gauge::BusyWorkers(ref t) => format!("jobsrv.workers.busy.{}", t).into(),
            Gauge::ReadyWorkers(ref t) => format!("jobsrv.workers.ready.{}", t).into(),
        }
    }
}

pub enum Histogram {
    JobCompletionTime(PackageTarget),
}

impl metrics::HistogramMetric for Histogram {}

impl metrics::Metric for Histogram {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Histogram::JobCompletionTime(ref t) => format!("jobsrv.completion_time.{}", t).into(),
        }
    }
}
