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

use bldr_core::metrics;
use std::borrow::Cow;

pub enum Counter {
    CompletedJobs,
    FailedJobs,
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::CompletedJobs => "jobsrv.completed".into(),
            Counter::FailedJobs => "jobsrv.failed".into(),
        }
    }
}

pub enum Gauge {
    WaitingJobs,
    WorkingJobs,
    Workers,
    BusyWorkers,
    ReadyWorkers,
}

impl metrics::GaugeMetric for Gauge {}

impl metrics::Metric for Gauge {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Gauge::WaitingJobs => "jobsrv.waiting".into(),
            Gauge::WorkingJobs => "jobsrv.working".into(),
            Gauge::Workers => "jobsrv.workers".into(),
            Gauge::BusyWorkers => "jobsrv.workers.busy".into(),
            Gauge::ReadyWorkers => "jobsrv.workers.ready".into(),
        }
    }
}

pub enum Histogram {
    JobCompletionTime,
}

impl metrics::HistogramMetric for Histogram {}

impl metrics::Metric for Histogram {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Histogram::JobCompletionTime => "jobsrv.completion_time".into(),
        }
    }
}
