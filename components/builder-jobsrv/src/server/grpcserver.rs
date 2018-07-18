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
use data_store::DataStore;
use error::Result;
use futures::Future;
use grpcio::{Environment, RpcContext, RpcStatus, RpcStatusCode, ServerBuilder, UnarySink};
use num_cpus;
use protocol::jobsrv;
use std::net::IpAddr;
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};

use config::Config;

use server::jobservice::{
    HelloRequest, HelloResponse, JobGraphPackageStats, JobGraphPackageStatsGet,
};
use server::jobservice_grpc::{create_job_service, JobService};

#[derive(Clone)]
struct JobServiceImpl {
    data_store: DataStore,
}

impl JobService for JobServiceImpl {
    fn say_hello(&self, ctx: RpcContext, req: HelloRequest, sink: UnarySink<HelloResponse>) {
        let msg = format!("Hello {}", req.get_greeting());
        let mut resp = HelloResponse::new();
        resp.set_reply(msg);
        let f = sink.success(resp)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }

    // TODO : Once we have the service proto properly wired up in
    // builder-protocol, we can remove the duplicate messages in the
    // function below
    fn get_job_graph_package_stats(
        &self,
        ctx: RpcContext,
        req: JobGraphPackageStatsGet,
        sink: UnarySink<JobGraphPackageStats>,
    ) {
        debug!("Getting stats for origin: {}", req.get_origin());

        let mut msg = jobsrv::JobGraphPackageStatsGet::new();
        msg.set_origin(req.get_origin().to_string());

        match self.data_store.get_job_graph_package_stats(&msg) {
            Ok(package_stats) => {
                debug!("Got package stats: {:?}", package_stats);

                let mut resp = JobGraphPackageStats::new();
                resp.set_plans(package_stats.get_plans());
                resp.set_builds(package_stats.get_builds());
                resp.set_unique_packages(package_stats.get_unique_packages());

                let f = sink.success(resp)
                    .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
                ctx.spawn(f)
            }
            Err(err) => {
                warn!(
                    "Unable to retrieve package stats for {}, err: {:?}",
                    msg.get_origin(),
                    err
                );
                let f = sink.fail(RpcStatus::new(
                    RpcStatusCode::NotFound,
                    Some("jb:job-graph-package-stats-get:1".to_string()),
                )).map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
                ctx.spawn(f)
            }
        };
    }
}

pub struct GrpcServer {
    addr: IpAddr,
    port: u16,
    data_store: DataStore,
}

impl GrpcServer {
    pub fn new(data_store: DataStore, addr: IpAddr, port: u16) -> Self {
        GrpcServer {
            addr: addr,
            port: port,
            data_store: data_store,
        }
    }

    pub fn start(cfg: &Config, data_store: DataStore) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut grpc_server = Self::new(data_store, cfg.net.grpc_listen, cfg.net.grpc_port);
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
        // Default connection queues to number of cores available
        // TODO: Make this configurable later if needed
        let cq_count = num_cpus::get();
        info!("Starting GRPC server with {} threads", cq_count);

        let env = Arc::new(Environment::new(cq_count));
        let instance = JobServiceImpl {
            data_store: self.data_store.clone(),
        };
        let service = create_job_service(instance);
        let mut server = ServerBuilder::new(env)
            .register_service(service)
            .bind(self.addr.to_string(), self.port)
            .build()
            .unwrap();
        server.start();
        for &(ref host, port) in server.bind_addrs() {
            info!("Listening on {}:{}", host, port);
        }
        rz.send(()).unwrap();

        loop {
            thread::park();
        }
    }
}
