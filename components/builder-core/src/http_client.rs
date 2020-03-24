use std::ops::Deref;

use std::{fs,
          path::Path};

use reqwest::{blocking::Client,
              header::{HeaderMap,
                       HeaderName,
                       HeaderValue,
                       ACCEPT,
                       CONTENT_TYPE,
                       USER_AGENT},
              Certificate,
              IntoUrl,
              Proxy};

use url::Url;

use crate::error::{Error,
                   Result};

const BLDR_USER_AGENT: &str = "Habitat-Builder";
const APPLICATION_JSON: &str = "application/json";
const FORM_URL_ENCODED: &str = "application/x-www-form-urlencoded";
const GITHUB_JSON: &str = "application/json, application/vnd.github.v3+json, \
                           application/vnd.github.machine-man-preview+json";
const X_FILENAME: &str = "x-filename";

lazy_static::lazy_static! {
    pub static ref USER_AGENT_BLDR: (HeaderName, HeaderValue) = (USER_AGENT, HeaderValue::from_static(BLDR_USER_AGENT));
    pub static ref ACCEPT_APPLICATION_JSON: (HeaderName, HeaderValue) = (ACCEPT, HeaderValue::from_static(APPLICATION_JSON));
    pub static ref ACCEPT_GITHUB_JSON: (HeaderName, HeaderValue) = (ACCEPT, HeaderValue::from_static(GITHUB_JSON));
    pub static ref CONTENT_TYPE_APPLICATION_JSON: (HeaderName, HeaderValue) = (CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
    pub static ref CONTENT_TYPE_FORM_URL_ENCODED: (HeaderName, HeaderValue) = (CONTENT_TYPE, HeaderValue::from_static(FORM_URL_ENCODED));
    pub static ref XFILENAME: HeaderName = HeaderName::from_static(X_FILENAME);
}

#[derive(Clone)]
pub struct HttpClient(Client);

impl HttpClient {
    pub fn new<T>(url: T, headers: HeaderMap) -> Result<Self>
        where T: IntoUrl
    {
        let url = url.into_url().map_err(Error::HttpClient)?;
        let mut client = Client::builder().proxy(proxy_for(&url)?)
                                          .default_headers(headers);

        client = certificates()?.into_iter()
                                .fold(client, |client, cert| client.add_root_certificate(cert));

        Ok(HttpClient(client.build()?))
    }
}

impl Deref for HttpClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target { &self.0 }
}

fn proxy_for(url: &Url) -> reqwest::Result<Proxy> {
    trace!("Checking proxy for url: {:?}", url);

    if let Some(proxy_url) = env_proxy::for_url(url).to_string() {
        match url.scheme() {
            "http" => {
                debug!("Setting http_proxy to {}", proxy_url);
                Proxy::http(&proxy_url)
            }
            "https" => {
                debug!("Setting https proxy to {}", proxy_url);
                Proxy::https(&proxy_url)
            }
            _ => unimplemented!(),
        }
    } else {
        debug!("No proxy configured for url: {:?}", url);
        Ok(Proxy::custom(|_| None::<Url>))
    }
}

/// We need a set of root certificates when connected to SSL/TLS web endpoints.
///
/// Builder (and on-prem Builder) will generally have a SSL_CERT_FILE environment
/// configured in the running environment that points to an installed core/cacerts
/// PEM file. The SSL_CERT_FILE environment variable will be respected by default.
///
/// In addtion, other certs files (for example self-signed certs) that are found in
/// the SSL cache directory (/hab/cache/ssl) will also get loaded into the root certs list.
/// Both PEM and DER formats are supported. All files will be assumed to be one of the
/// supported formats, and any errors will be ignored silently (other than debug logging)
fn certificates() -> Result<Vec<Certificate>> {
    let mut certificates = Vec::new();

    // Note: we don't use the fs::cache_ssl_path because it defaults to using the
    // $HOME environment which is set to the /hab/svc/... path for Builder, which
    // is not what we want to be using here
    process_cache_dir("/hab/cache/ssl", &mut certificates);
    Ok(certificates)
}

fn process_cache_dir<P>(cache_path: P, mut certificates: &mut Vec<Certificate>)
    where P: AsRef<Path>
{
    debug!("Processing cache directory: {:?}", cache_path.as_ref());

    match fs::read_dir(cache_path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            process_cert_file(&mut certificates, &path);
                        }
                    }
                    Err(err) => debug!("Unable to read cache entry, err = {:?}", err),
                }
            }
        }
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                debug!("Unable to read cache directory, err = {:?}", err)
            }
        }
    }
}

fn process_cert_file(certificates: &mut Vec<Certificate>, file_path: &Path) {
    debug!("Processing cert file: {}", file_path.display());

    match cert_from_file(&file_path) {
        Ok(cert) => certificates.push(cert),
        Err(err) => {
            debug!("Unable to process cert file: {}, err={:?}",
                   file_path.display(),
                   err)
        }
    }
}

fn cert_from_file(file_path: &Path) -> Result<Certificate> {
    let buf = fs::read(file_path).map_err(Error::IO)?;

    Certificate::from_pem(&buf).or_else(|_| Certificate::from_der(&buf))
                               .map_err(Error::HttpClient)
}
