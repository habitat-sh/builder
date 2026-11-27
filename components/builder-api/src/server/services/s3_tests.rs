//! Exercise the real SDK request sequence, including failure cleanup.
use super::*;
use std::{io::BufRead,
          net::{TcpListener,
                TcpStream},
          sync::{Arc,
                 Mutex},
          time::{Duration,
                 Instant}};

#[derive(Clone, Copy, PartialEq)]
enum Failure {
    None,
    Part,
    Complete,
}

#[derive(Debug)]
struct Request {
    method:     String,
    path:       String,
    length:     usize,
    completion: String,
}

fn read_request(stream: &mut TcpStream) -> Request {
    stream.set_read_timeout(Some(Duration::from_secs(15)))
          .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let mut fields = line.split_whitespace();
    let method = fields.next().unwrap().to_owned();
    let path = fields.next().unwrap().to_owned();
    let mut content_length = 0;
    let mut decoded_length = None;
    let mut chunked = false;
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        let (name, value) = line.split_once(':').unwrap();
        match name.to_ascii_lowercase().as_str() {
            "content-length" => content_length = value.trim().parse().unwrap(),
            "x-amz-decoded-content-length" => decoded_length = Some(value.trim().parse().unwrap()),
            "transfer-encoding" => chunked = value.trim() == "chunked",
            "expect" if value.trim() == "100-continue" => {
                stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").unwrap();
            }
            _ => {}
        }
    }
    let mut body = Vec::new();
    if chunked {
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            let size = usize::from_str_radix(line.trim().split(';').next().unwrap(), 16).unwrap();
            if size == 0 {
                break;
            }
            let old = body.len();
            body.resize(old + size, 0);
            reader.read_exact(&mut body[old..]).unwrap();
            let mut crlf = [0; 2];
            reader.read_exact(&mut crlf).unwrap();
        }
    } else {
        body.resize(content_length, 0);
        reader.read_exact(&mut body).unwrap();
    }
    let completion = if method == "POST" && path.contains("uploadId=") {
        String::from_utf8(body.clone()).unwrap()
    } else {
        String::new()
    };
    Request { method,
              path,
              length: decoded_length.unwrap_or(body.len()),
              completion }
}

fn storage_server(failure: Failure) -> (String, Arc<Mutex<Vec<Request>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("accept: {}", e),
            };
            let request = read_request(&mut stream);
            let failed = (failure == Failure::Part
                          && request.method == "PUT"
                          && request.path.contains("partNumber="))
                         || (failure == Failure::Complete
                             && request.method == "POST"
                             && request.path.contains("uploadId="));
            let terminal = request.method == "HEAD" || request.method == "DELETE";
            let body = if failed {
                "<Error><Code>InvalidRequest</Code><Message>Injected failure</Message></Error>"
            } else if request.method == "POST" && request.path.contains("uploads") {
                "<InitiateMultipartUploadResult><Bucket>artifacts</Bucket><Key>artifact</\
                 Key><UploadId>upload-1</UploadId></InitiateMultipartUploadResult>"
            } else if request.method == "POST" {
                "<CompleteMultipartUploadResult><Bucket>artifacts</Bucket><Key>artifact</\
                 Key><ETag>etag</ETag></CompleteMultipartUploadResult>"
            } else {
                ""
            };
            captured.lock().unwrap().push(request);
            let response = format!("HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: \
                                    application/xml\r\nETag: \"part-etag\"\r\nConnection: \
                                    close\r\n\r\n{}",
                                   if failed { "400 Bad Request" } else { "200 OK" },
                                   body.len(),
                                   body);
            stream.write_all(response.as_bytes()).unwrap();
            if terminal {
                break;
            }
        }
    });
    (endpoint, requests)
}

fn handler(backend: S3Backend, endpoint: String) -> S3Handler {
    let mut handler = S3Handler::new(S3Cfg { backend,
                                             endpoint,
                                             key_id: "test-key".into(),
                                             secret_key: "test-secret".into(),
                                             bucket_name: "artifacts".into() });
    handler.client =
        S3Client::from_conf(handler.client
                                   .config()
                                   .to_builder()
                                   .retry_config(aws_sdk_s3::config::retry::RetryConfig::disabled())
                                   .build());
    handler
}

struct UploadFile(PathBuf);
impl UploadFile {
    fn new(size: usize) -> Self {
        let path = std::env::temp_dir().join(format!("builder-s3-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, vec![b'Z'; size]).unwrap();
        Self(path)
    }
}
impl Drop for UploadFile {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

#[tokio::test]
async fn single_and_multipart_boundaries_use_the_existing_s3_client() {
    for size in [MULTIPART_PART_SIZE - 1,
                 MULTIPART_PART_SIZE,
                 MULTIPART_PART_SIZE + 1,
                 2 * MULTIPART_PART_SIZE,
                 2 * MULTIPART_PART_SIZE + 1]
    {
        let (endpoint, requests) = storage_server(Failure::None);
        let handler = handler(S3Backend::Minio, endpoint);
        let file = UploadFile::new(size);
        handler.upload(&file.0,
                       &"acme/test/1.0/20200101000000".parse().unwrap(),
                       "x86_64-linux".parse().unwrap())
               .await
               .unwrap();
        let requests = requests.lock().unwrap();
        assert!(requests.iter()
                        .all(|r| r.path.starts_with("/artifacts/acme/test/")));
        if size < MULTIPART_PART_SIZE {
            assert_eq!(requests.iter().filter(|r| r.method == "PUT").count(), 1);
            assert_eq!(requests[0].length, size);
        } else {
            let parts: Vec<_> = requests.iter().filter(|r| r.method == "PUT").collect();
            assert_eq!(parts.len(), size.div_ceil(MULTIPART_PART_SIZE));
            assert!(parts.iter()
                         .all(|r| r.length > 0 && r.length <= MULTIPART_PART_SIZE));
            assert_eq!(parts.iter().map(|r| r.length).sum::<usize>(), size);
            let completed = requests.iter().find(|r| !r.completion.is_empty()).unwrap();
            assert_eq!(completed.completion.matches("<PartNumber>").count(),
                       parts.len());
        }
    }
}

#[tokio::test]
async fn multipart_part_and_completion_failures_abort_the_session() {
    for failure in [Failure::Part, Failure::Complete] {
        let (endpoint, requests) = storage_server(failure);
        let handler = handler(S3Backend::Minio, endpoint);
        let file = UploadFile::new(MULTIPART_PART_SIZE);
        let error = handler.upload(&file.0,
                                   &"acme/test/1.0/20200101000000".parse().unwrap(),
                                   "x86_64-linux".parse().unwrap())
                           .await
                           .unwrap_err();
        assert!(format!("{:?}", error).contains("Injected failure"));
        let requests = requests.lock().unwrap();
        let last = requests.last().unwrap();
        assert_eq!(last.method, "DELETE");
        assert!(last.path.contains("uploadId=upload-1"));
    }
}

#[test]
fn existing_s3_configuration_supports_custom_endpoints_without_changing_artifactory() {
    use crate::{bldr_core::config::ConfigFile,
                config::Config};
    for (name, endpoint, backend) in [("aws", "eu-west-1", S3Backend::Aws),
                                      ("minio", "http://127.0.0.1:19090", S3Backend::Minio)]
    {
        let config = Config::from_raw(&format!(
            r#"
            [s3]
            backend = "{}"
            endpoint = "{}"
            key_id = "builder"
            secret_key = "test-secret"
            bucket_name = "artifacts"
            [artifactory]
            api_url = "http://artifactory:8080"
            api_key = "existing-key"
            repo = "existing-repo"
        "#,
            name, endpoint
        )).unwrap();
        assert_eq!(config.s3.backend, backend);
        assert_eq!(config.s3.endpoint, endpoint);
        assert_eq!(config.s3.key_id, "builder");
        assert_eq!(config.s3.secret_key, "test-secret");
        assert_eq!(config.s3.bucket_name, "artifacts");
        assert_eq!(config.artifactory.api_url, "http://artifactory:8080");
        assert_eq!(config.artifactory.api_key, "existing-key");
        assert_eq!(config.artifactory.repo, "existing-repo");
    }
    assert_eq!(Config::from_raw("").unwrap().s3.backend, S3Backend::Minio);
}

#[test]
fn aws_and_minio_signing_regions_are_unchanged() {
    for (backend, endpoint, expected_region) in
        [(S3Backend::Aws, "eu-west-1", "eu-west-1"),
         (S3Backend::Minio, "http://127.0.0.1:9000", "minio_s3")]
    {
        let handler = handler(backend, endpoint.into());
        assert_eq!(handler.client.config().region().unwrap().as_ref(),
                   expected_region);
    }
}

#[tokio::test]
#[ignore = "requires a running SeaweedFS endpoint and explicit test credentials"]
async fn live_seaweedfs_roundtrip_and_delete_for_single_and_multipart_uploads() {
    use futures::StreamExt;
    let endpoint = std::env::var("BUILDER_TEST_S3_ENDPOINT").expect("test endpoint");
    let bucket = format!("builder-sdk-{}", uuid::Uuid::new_v4());
    let handler =
        S3Handler::new(S3Cfg { backend: S3Backend::Minio,
                               endpoint,
                               key_id:
                                   std::env::var("BUILDER_TEST_S3_KEY_ID").expect("test access key"),
                               secret_key:
                                   std::env::var("BUILDER_TEST_S3_SECRET_KEY").expect("test secret"),
                               bucket_name: bucket.clone() });
    handler.client
           .create_bucket()
           .bucket(&bucket)
           .send()
           .await
           .unwrap();
    for size in [1024, 2 * MULTIPART_PART_SIZE + 1] {
        let file = UploadFile::new(size);
        let ident = format!("storage-test/roundtrip/{}/20200101000000", size).parse()
                                                                             .unwrap();
        let target = "x86_64-linux".parse().unwrap();
        handler.upload(&file.0, &ident, target).await.unwrap();
        assert_eq!(handler.size_of(&ident, target).await.unwrap(), size as i64);
        let (_, mut stream) = handler.download_stream(&ident, target).await.unwrap();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            bytes.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(bytes, std::fs::read(&file.0).unwrap());
        handler.delete(&ident, target).await.unwrap();
        let error = handler.client
                           .head_object()
                           .bucket(&bucket)
                           .key(s3_key(&ident, target).unwrap())
                           .send()
                           .await
                           .unwrap_err();
        assert!(error.as_service_error().unwrap().is_not_found());
    }
    let pending = handler.client
                         .list_multipart_uploads()
                         .bucket(&bucket)
                         .send()
                         .await
                         .unwrap();
    assert!(pending.uploads().is_empty());
    handler.client
           .delete_bucket()
           .bucket(bucket)
           .send()
           .await
           .unwrap();
}
