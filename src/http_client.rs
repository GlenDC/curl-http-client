use std::{path::Path, time::Duration};

use async_curl::async_curl::AsyncCurl;
use curl::easy::{Auth, Easy2};
use derive_deref_rs::Deref;
use http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, Method, StatusCode};

use crate::{collector::Collector, error::Error, request::HttpRequest, response::HttpResponse};

/// A type-state struct in building the HttpClient.
pub struct Build;
/// A type-state struct in building the HttpClient.
pub struct Perform;

/// The HTTP Client struct that wraps curl Easy2.
pub struct HttpClient<S> {
    /// This is the the actor handler that can be cloned to be able to handle multiple request sender
    /// and a single consumer that is spawned in the background upon creation of this object to be able to achieve
    /// non-blocking I/O during curl perform.
    curl: AsyncCurl<Collector>,
    /// The `Easy2<Collector>` is the Easy2 from curl-rust crate wrapped in this struct to be able to do
    /// asynchronous task during.
    easy: Easy2<Collector>,
    /// This is a type-state builder pattern to help programmers not to mis-used when buding curl settings before perform
    /// operation.
    _state: S,
}

impl HttpClient<Build> {
    /// Creates a new HTTP Client.
    ///
    /// The [`AsyncCurl<Collector>`](https://docs.rs/async-curl/latest/async_curl/async_curl/struct.AsyncCurl.html) is the actor handler that can be cloned to be able to handle multiple request sender
    /// and a single consumer that is spawned in the background upon creation of this object to be able to achieve
    /// non-blocking I/O during curl perform.
    ///
    /// The Collector is the type of container whether via RAM or via File.
    pub fn new(curl: AsyncCurl<Collector>, collector: Collector) -> Self {
        Self {
            curl,
            easy: Easy2::new(collector),
            _state: Build,
        }
    }

    /// Sets the HTTP request.
    ///
    /// The HttpRequest can be customized by the caller byt setting the Url, Method Type,
    /// Headers and the Body.
    pub fn request(mut self, request: HttpRequest) -> Result<HttpClient<Perform>, Error> {
        self.easy.url(&request.url.to_string()[..]).map_err(|e| {
            eprintln!("{:?}", e);
            Error::Curl(e.to_string())
        })?;

        let mut headers = curl::easy::List::new();
        request.headers.iter().try_for_each(|(name, value)| {
            headers
                .append(&format!(
                    "{}: {}",
                    name,
                    value.to_str().map_err(|_| Error::Other(format!(
                        "invalid {} header value {:?}",
                        name,
                        value.as_bytes()
                    )))?
                ))
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    Error::Curl(e.to_string())
                })
        })?;

        self.easy.http_headers(headers).map_err(|e| {
            eprintln!("{:?}", e);
            Error::Curl(e.to_string())
        })?;

        match request.method {
            Method::POST => {
                self.easy
                    .post(true)
                    .map_err(|e| Error::Curl(e.to_string()))?;
                if let Some(body) = request.body {
                    self.easy.post_field_size(body.len() as u64).map_err(|e| {
                        eprintln!("{:?}", e);
                        Error::Curl(e.to_string())
                    })?;
                    self.easy.post_fields_copy(body.as_slice()).map_err(|e| {
                        eprintln!("{:?}", e);
                        Error::Curl(e.to_string())
                    })?;
                }
            }
            Method::GET => {
                self.easy
                    .get(true)
                    .map_err(|e| Error::Curl(e.to_string()))?;
            }
            Method::PUT => {
                self.easy
                    .upload(true)
                    .map_err(|e| Error::Curl(e.to_string()))?;
            }
            _ => {
                // TODO: For Future improvements to handle other Methods
                unimplemented!();
            }
        }
        Ok(HttpClient::<Perform> {
            curl: self.curl,
            easy: self.easy,
            _state: Perform,
        })
    }

    /// Set a point to resume transfer from
    ///
    /// Specify the offset in bytes you want the transfer to start from.
    ///
    /// By default this option is 0 and corresponds to
    /// `CURLOPT_RESUME_FROM_LARGE`.
    pub fn resume_from(mut self, offset: BytesOffset) -> Result<Self, Error> {
        self.easy
            .resume_from(*offset as u64)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Rate limit data download speed
    ///
    /// If a download exceeds this speed (counted in bytes per second) on
    /// cumulative average during the transfer, the transfer will pause to keep
    /// the average rate less than or equal to the parameter value.
    ///
    /// By default this option is not set (unlimited speed) and corresponds to
    /// `CURLOPT_MAX_RECV_SPEED_LARGE`.
    pub fn download_speed(mut self, speed: BytesPerSec) -> Result<Self, Error> {
        self.easy
            .max_recv_speed(*speed)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Set the size of the input file to send off.
    ///
    /// By default this option is not set and corresponds to
    /// `CURLOPT_INFILESIZE_LARGE`.
    pub fn upload_file_size(mut self, size: FileSize) -> Result<Self, Error> {
        self.easy
            .in_filesize(*size as u64)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Rate limit data upload speed
    ///
    /// If an upload exceeds this speed (counted in bytes per second) on
    /// cumulative average during the transfer, the transfer will pause to keep
    /// the average rate less than or equal to the parameter value.
    ///
    /// By default this option is not set (unlimited speed) and corresponds to
    /// `CURLOPT_MAX_SEND_SPEED_LARGE`.
    pub fn upload_speed(mut self, speed: BytesPerSec) -> Result<Self, Error> {
        self.easy
            .max_send_speed(*speed)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    // =========================================================================
    // Names and passwords

    /// Configures the username to pass as authentication for this connection.
    ///
    /// By default this value is not set and corresponds to `CURLOPT_USERNAME`.
    pub fn username(mut self, user: &str) -> Result<Self, Error> {
        self.easy
            .username(user)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Configures the password to pass as authentication for this connection.
    ///
    /// By default this value is not set and corresponds to `CURLOPT_PASSWORD`.
    pub fn password(mut self, pass: &str) -> Result<Self, Error> {
        self.easy
            .password(pass)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Set HTTP server authentication methods to try
    ///
    /// If more than one method is set, libcurl will first query the site to see
    /// which authentication methods it supports and then pick the best one you
    /// allow it to use. For some methods, this will induce an extra network
    /// round-trip. Set the actual name and password with the `password` and
    /// `username` methods.
    ///
    /// For authentication with a proxy, see `proxy_auth`.
    ///
    /// By default this value is basic and corresponds to `CURLOPT_HTTPAUTH`.
    pub fn http_auth(mut self, auth: &Auth) -> Result<Self, Error> {
        self.easy
            .http_auth(auth)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Configures the port number to connect to, instead of the one specified
    /// in the URL or the default of the protocol.
    pub fn port(mut self, port: u16) -> Result<Self, Error> {
        self.easy
            .port(port)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    // /// Verify the certificate's status.
    // ///
    // /// This option determines whether libcurl verifies the status of the server
    // /// cert using the "Certificate Status Request" TLS extension (aka. OCSP
    // /// stapling).
    // ///
    // /// By default this option is set to `false` and corresponds to
    // /// `CURLOPT_SSL_VERIFYSTATUS`.
    // pub fn ssl_verify_status(&mut self, verify: bool) -> Result<(), Error> {
    //     self.setopt_long(curl_sys::CURLOPT_SSL_VERIFYSTATUS, verify as c_long)
    // }

    /// Specify the path to Certificate Authority (CA) bundle
    ///
    /// The file referenced should hold one or more certificates to verify the
    /// peer with.
    ///
    /// This option is by default set to the system path where libcurl's cacert
    /// bundle is assumed to be stored, as established at build time.
    ///
    /// If curl is built against the NSS SSL library, the NSS PEM PKCS#11 module
    /// (libnsspem.so) needs to be available for this option to work properly.
    ///
    /// By default this option is the system defaults, and corresponds to
    /// `CURLOPT_CAINFO`.
    pub fn cainfo<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        self.easy
            .cainfo(path)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Specify directory holding CA certificates
    ///
    /// Names a directory holding multiple CA certificates to verify the peer
    /// with. If libcurl is built against OpenSSL, the certificate directory
    /// must be prepared using the openssl c_rehash utility. This makes sense
    /// only when used in combination with the `ssl_verify_peer` option.
    ///
    /// By default this option is not set and corresponds to `CURLOPT_CAPATH`.
    pub fn capath<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        self.easy
            .capath(path)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Configures the proxy username to pass as authentication for this
    /// connection.
    ///
    /// By default this value is not set and corresponds to
    /// `CURLOPT_PROXYUSERNAME`.
    pub fn proxy_username(mut self, user: &str) -> Result<Self, Error> {
        self.easy
            .proxy_username(user)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Configures the proxy password to pass as authentication for this
    /// connection.
    ///
    /// By default this value is not set and corresponds to
    /// `CURLOPT_PROXYPASSWORD`.
    pub fn proxy_password(mut self, pass: &str) -> Result<Self, Error> {
        self.easy
            .proxy_password(pass)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Set HTTP proxy authentication methods to try
    ///
    /// If more than one method is set, libcurl will first query the site to see
    /// which authentication methods it supports and then pick the best one you
    /// allow it to use. For some methods, this will induce an extra network
    /// round-trip. Set the actual name and password with the `proxy_password`
    /// and `proxy_username` methods.
    ///
    /// By default this value is basic and corresponds to `CURLOPT_PROXYAUTH`.
    pub fn proxy_auth(mut self, auth: &Auth) -> Result<Self, Error> {
        self.easy
            .proxy_auth(auth)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Provide the URL of a proxy to use.
    ///
    /// By default this option is not set and corresponds to `CURLOPT_PROXY`.
    pub fn proxy(mut self, url: &str) -> Result<Self, Error> {
        self.easy
            .proxy(url)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Provide port number the proxy is listening on.
    ///
    /// By default this option is not set (the default port for the proxy
    /// protocol is used) and corresponds to `CURLOPT_PROXYPORT`.
    pub fn proxy_port(mut self, port: u16) -> Result<Self, Error> {
        self.easy
            .proxy_port(port)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Set CA certificate to verify peer against for proxy.
    ///
    /// By default this value is not set and corresponds to
    /// `CURLOPT_PROXY_CAINFO`.
    pub fn proxy_cainfo(mut self, cainfo: &str) -> Result<Self, Error> {
        self.easy
            .proxy_cainfo(cainfo)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Specify a directory holding CA certificates for proxy.
    ///
    /// The specified directory should hold multiple CA certificates to verify
    /// the HTTPS proxy with. If libcurl is built against OpenSSL, the
    /// certificate directory must be prepared using the OpenSSL `c_rehash`
    /// utility.
    ///
    /// By default this value is not set and corresponds to
    /// `CURLOPT_PROXY_CAPATH`.
    pub fn proxy_capath<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        self.easy
            .proxy_capath(path)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Follow HTTP 3xx redirects.
    ///
    /// Indicates whether any `Location` headers in the response should get
    /// followed.
    ///
    /// By default this option is `false` and corresponds to
    /// `CURLOPT_FOLLOWLOCATION`.
    pub fn follow_location(mut self, enable: bool) -> Result<Self, Error> {
        self.easy
            .follow_location(enable)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Timeout for the connect phase
    ///
    /// This is the maximum time that you allow the connection phase to the
    /// server to take. This only limits the connection phase, it has no impact
    /// once it has connected.
    ///
    /// By default this value is 300 seconds and corresponds to
    /// `CURLOPT_CONNECTTIMEOUT_MS`.
    pub fn connect_timeout(mut self, timeout: Duration) -> Result<Self, Error> {
        self.easy
            .connect_timeout(timeout)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    // =========================================================================
    // Connection Options

    /// Set maximum time the request is allowed to take.
    ///
    /// Normally, name lookups can take a considerable time and limiting
    /// operations to less than a few minutes risk aborting perfectly normal
    /// operations.
    ///
    /// If libcurl is built to use the standard system name resolver, that
    /// portion of the transfer will still use full-second resolution for
    /// timeouts with a minimum timeout allowed of one second.
    ///
    /// In unix-like systems, this might cause signals to be used unless
    /// `nosignal` is set.
    ///
    /// Since this puts a hard limit for how long a request is allowed to
    /// take, it has limited use in dynamic use cases with varying transfer
    /// times. You are then advised to explore `low_speed_limit`,
    /// `low_speed_time` or using `progress_function` to implement your own
    /// timeout logic.
    ///
    /// By default this option is not set and corresponds to
    /// `CURLOPT_TIMEOUT_MS`.
    pub fn timeout(mut self, timeout: Duration) -> Result<Self, Error> {
        self.easy
            .timeout(timeout)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    // =========================================================================
    // Behavior options

    /// Configures this handle to have verbose output to help debug protocol
    /// information.
    ///
    /// By default output goes to stderr, but the `stderr` function on this type
    /// can configure that. You can also use the `debug_function` method to get
    /// all protocol data sent and received.
    ///
    /// By default, this option is `false`.
    pub fn verbose(mut self, verbose: bool) -> Result<Self, Error> {
        self.easy
            .verbose(verbose)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }

    /// Indicates whether header information is streamed to the output body of
    /// this request.
    ///
    /// This option is only relevant for protocols which have header metadata
    /// (like http or ftp). It's not generally possible to extract headers
    /// from the body if using this method, that use case should be intended for
    /// the `header_function` method.
    ///
    /// To set HTTP headers, use the `http_header` method.
    ///
    /// By default, this option is `false` and corresponds to
    /// `CURLOPT_HEADER`.
    pub fn show_header(mut self, show: bool) -> Result<Self, Error> {
        self.easy
            .show_header(show)
            .map_err(|e| Error::Curl(e.to_string()))?;
        Ok(self)
    }
}

impl HttpClient<Perform> {
    /// This will perform the curl operation asynchronously.
    /// This becomes a non-blocking I/O since the actual perform operation is done
    /// at the actor side.
    pub async fn perform(self) -> Result<HttpResponse, Error> {
        let mut easy = self.curl.send_request(self.easy).await.map_err(|e| {
            eprintln!("{:?}", e);
            Error::Curl(e.to_string())
        })?;

        let data = easy.get_ref().get_response_body().take();
        let status_code = easy.response_code().map_err(|e| {
            eprintln!("{:?}", e);
            Error::Curl(e.to_string())
        })? as u16;
        let response_header = easy
            .content_type()
            .map_err(|e| {
                eprintln!("{:?}", e);
                Error::Curl(e.to_string())
            })?
            .map(|content_type| {
                Ok(vec![(
                    CONTENT_TYPE,
                    HeaderValue::from_str(content_type).map_err(|err| {
                        eprintln!("{:?}", err);
                        Error::Curl(err.to_string())
                    })?,
                )]
                .into_iter()
                .collect::<HeaderMap>())
            })
            .transpose()?
            .unwrap_or_else(HeaderMap::new);

        Ok(HttpResponse {
            status_code: StatusCode::from_u16(status_code).map_err(|err| {
                eprintln!("{:?}", err);
                Error::Curl(err.to_string())
            })?,
            headers: response_header,
            body: data,
        })
    }
}

/// A strong type unit when setting download speed and upload speed
/// in bytes per second.
#[derive(Deref)]
pub struct BytesPerSec(u64);

impl From<u64> for BytesPerSec {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// A strong type unit when offsetting especially in resuming download
#[derive(Deref)]
pub struct BytesOffset(usize);

impl From<usize> for BytesOffset {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// A strong type unit when setting a file size.
#[derive(Deref)]
pub struct FileSize(usize);

impl From<usize> for FileSize {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
