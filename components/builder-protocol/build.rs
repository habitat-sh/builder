use std::fs;

fn protocol_files() -> Vec<String> {
    let mut files = vec![];
    for entry in fs::read_dir("protocols").unwrap() {
        let file = entry.unwrap();
        // skip vim temp files
        if file.file_name().to_str().unwrap().starts_with('.') {
            continue;
        }
        if file.metadata().unwrap().is_file() {
            files.push(file.path().to_str().unwrap().into());
        }
    }
    files
}

fn main() {
    let protocols = protocol_files();
    let protocols: Vec<&str> = protocols.iter().map(AsRef::as_ref).collect();

    protoc_rust::Codegen::new().out_dir("src/message")
                               .inputs(&protocols)
                               .includes(&["protocols"])
                               .run()
                               .expect("Failed to run protoc, please check that it is available \
                                        on your PATH and that the src/message folder is writable");
}
