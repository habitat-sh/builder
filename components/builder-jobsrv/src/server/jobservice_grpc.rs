// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_JOB_SERVICE_SAY_HELLO: ::grpcio::Method<super::jobservice::HelloRequest, super::jobservice::HelloResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/jobservice.JobService/SayHello",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_JOB_SERVICE_GET_JOB_GRAPH_PACKAGE_STATS: ::grpcio::Method<super::jobservice::JobGraphPackageStatsGet, super::jobservice::JobGraphPackageStats> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/jobservice.JobService/GetJobGraphPackageStats",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

pub struct JobServiceClient {
    client: ::grpcio::Client,
}

impl JobServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        JobServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn say_hello_opt(&self, req: &super::jobservice::HelloRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::jobservice::HelloResponse> {
        self.client.unary_call(&METHOD_JOB_SERVICE_SAY_HELLO, req, opt)
    }

    pub fn say_hello(&self, req: &super::jobservice::HelloRequest) -> ::grpcio::Result<super::jobservice::HelloResponse> {
        self.say_hello_opt(req, ::grpcio::CallOption::default())
    }

    pub fn say_hello_async_opt(&self, req: &super::jobservice::HelloRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::jobservice::HelloResponse>> {
        self.client.unary_call_async(&METHOD_JOB_SERVICE_SAY_HELLO, req, opt)
    }

    pub fn say_hello_async(&self, req: &super::jobservice::HelloRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::jobservice::HelloResponse>> {
        self.say_hello_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_job_graph_package_stats_opt(&self, req: &super::jobservice::JobGraphPackageStatsGet, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::jobservice::JobGraphPackageStats> {
        self.client.unary_call(&METHOD_JOB_SERVICE_GET_JOB_GRAPH_PACKAGE_STATS, req, opt)
    }

    pub fn get_job_graph_package_stats(&self, req: &super::jobservice::JobGraphPackageStatsGet) -> ::grpcio::Result<super::jobservice::JobGraphPackageStats> {
        self.get_job_graph_package_stats_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_job_graph_package_stats_async_opt(&self, req: &super::jobservice::JobGraphPackageStatsGet, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::jobservice::JobGraphPackageStats>> {
        self.client.unary_call_async(&METHOD_JOB_SERVICE_GET_JOB_GRAPH_PACKAGE_STATS, req, opt)
    }

    pub fn get_job_graph_package_stats_async(&self, req: &super::jobservice::JobGraphPackageStatsGet) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::jobservice::JobGraphPackageStats>> {
        self.get_job_graph_package_stats_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait JobService {
    fn say_hello(&self, ctx: ::grpcio::RpcContext, req: super::jobservice::HelloRequest, sink: ::grpcio::UnarySink<super::jobservice::HelloResponse>);
    fn get_job_graph_package_stats(&self, ctx: ::grpcio::RpcContext, req: super::jobservice::JobGraphPackageStatsGet, sink: ::grpcio::UnarySink<super::jobservice::JobGraphPackageStats>);
}

pub fn create_job_service<S: JobService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_JOB_SERVICE_SAY_HELLO, move |ctx, req, resp| {
        instance.say_hello(ctx, req, resp)
    });
    let instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_JOB_SERVICE_GET_JOB_GRAPH_PACKAGE_STATS, move |ctx, req, resp| {
        instance.get_job_graph_package_stats(ctx, req, resp)
    });
    builder.build()
}
