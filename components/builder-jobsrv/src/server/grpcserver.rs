// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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
use error::Result;
use futures::Future;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};

use server::jobservice::{
    HelloRequest, HelloResponse, JobGraphPackageStats, JobGraphPackageStatsGet,
};
use server::jobservice_grpc::{create_job_service, JobService};

#[derive(Clone)]
struct JobServiceImpl;

impl JobService for JobServiceImpl {
    fn say_hello(&self, ctx: RpcContext, req: HelloRequest, sink: UnarySink<HelloResponse>) {
        let msg = format!("Hello {}", req.get_greeting());
        let mut resp = HelloResponse::new();
        resp.set_reply(msg);
        let f = sink.success(resp)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }

    fn get_job_graph_package_stats(
        &self,
        ctx: RpcContext,
        req: JobGraphPackageStatsGet,
        sink: UnarySink<JobGraphPackageStats>,
    ) {
        let resp = JobGraphPackageStats::new();
        let f = sink.success(resp)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }
}

pub struct GrpcServer {}

impl GrpcServer {
    pub fn new() -> Self {
        GrpcServer {}
    }
    pub fn start() -> Result<JoinHandle<()>> {
        println!("Starting GRPC Server...");
        let (tx, rx) = mpsc::sync_channel(1);
        let mut grpc_server = Self::new();
        let handle = thread::Builder::new()
            .name("grpcserver".to_string())
            .spawn(move || {
                grpc_server.run(tx).unwrap();
            })
            .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("grpc server thread startup error, err={}", e),
        }
    }

    fn run(&mut self, rz: mpsc::SyncSender<()>) -> Result<()> {
        let env = Arc::new(Environment::new(1));
        let service = create_job_service(JobServiceImpl);
        let mut server = ServerBuilder::new(env)
            .register_service(service)
            .bind("127.0.0.1", 50051)
            .build()
            .unwrap();
        server.start();
        for &(ref host, port) in server.bind_addrs() {
            info!("listening on {}:{}", host, port);
        }
        rz.send(()).unwrap();

        loop {
            thread::park();
        }
    }
}
