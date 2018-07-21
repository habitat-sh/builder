mod protocols {
    extern crate pkg_config;
    extern crate protobuf_codegen;
    extern crate protoc;
    extern crate protoc_grpcio;
    extern crate protoc_rust;

    use self::protobuf_codegen::Customize;
    use std::fs;

    pub fn generate_protocols() {
        let protocols = protocol_files();

        if protoc::Protoc::from_env_path()
            .version()
            .expect("version")
            .is_3()
        {
            println!("cargo:rustc-cfg=proto3");
        }

        protoc_rust::run(protoc_rust::Args {
            out_dir: "src/message",
            input: protocols
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
            includes: &["src/protocols"],
            customize: Customize {
            ..Default::default()
            },
        }).expect(
            "Failed to run protoc, please check that it is available on your PATH, and that the src/message folder is writable",
        );

        protoc_grpcio::compile_grpc_protos(
            &["jobsrv.proto"],
            &["src/protocols"],
            &"src/message",
        ).expect("Failed to compile gRPC definitions, please check that protoc is available on your PATH, and that the src/message folder is writable");
    }

    fn protocol_files() -> Vec<String> {
        let mut files = vec![];
        for entry in fs::read_dir("src/protocols").unwrap() {
            let file = entry.unwrap();
            // skip vim temp files
            if file.file_name().to_str().unwrap().starts_with(".") {
                continue;
            }
            if file.metadata().unwrap().is_file() {
                files.push(file.path().to_str().unwrap().into());
            }
        }
        files
    }
}
