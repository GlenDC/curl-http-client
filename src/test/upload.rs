use std::fs;

use async_curl::async_curl::AsyncCurl;
use http::{HeaderMap, Method, StatusCode};
use url::Url;

use crate::collector::Collector;
use crate::http_client::HttpClient;
use crate::request::HttpRequest;
use crate::test::test_setup::{setup_test_environment, MockResponder, ResponderType};

#[tokio::test]
async fn test_upload() {
    let responder = MockResponder::new(ResponderType::File);
    let (server, tempdir) = setup_test_environment(responder).await;
    let target_url = Url::parse(format!("{}/test", server.uri()).as_str()).unwrap();

    let to_be_uploaded = tempdir.path().join("file_to_be_uploaded.jpg");
    fs::write(to_be_uploaded.as_path(), include_bytes!("sample.jpg")).unwrap();
    let file_size = fs::metadata(to_be_uploaded.as_path()).unwrap().len() as usize;

    let curl = AsyncCurl::new();
    let collector = Collector::File(to_be_uploaded, 0);
    let request = HttpRequest {
        url: target_url,
        method: Method::PUT,
        headers: HeaderMap::new(),
        body: None,
    };
    let response = HttpClient::new(curl, collector)
        .upload_file_size(file_size)
        .unwrap()
        .request(request)
        .unwrap()
        .perform()
        .await
        .unwrap();

    println!("Response: {:?}", response);
    assert_eq!(response.status_code, StatusCode::OK);
    assert_eq!(response.body, None);
}

#[tokio::test]
async fn test_upload_with_speed_control() {
    let responder = MockResponder::new(ResponderType::File);
    let (server, tempdir) = setup_test_environment(responder).await;
    let target_url = Url::parse(format!("{}/test", server.uri()).as_str()).unwrap();

    let to_be_uploaded = tempdir.path().join("file_to_be_uploaded.jpg");
    fs::write(to_be_uploaded.clone(), include_bytes!("sample.jpg")).unwrap();
    let file_size = fs::metadata(to_be_uploaded.as_path()).unwrap().len() as usize;

    let curl = AsyncCurl::new();
    let collector = Collector::File(to_be_uploaded.clone(), 0);
    let request = HttpRequest {
        url: target_url,
        method: Method::PUT,
        headers: HeaderMap::new(),
        body: None,
    };
    let response = HttpClient::new(curl, collector)
        .upload_file_size(file_size)
        .unwrap()
        .upload_speed(4000000)
        .unwrap()
        .request(request)
        .unwrap()
        .perform()
        .await
        .unwrap();

    println!("Response: {:?}", response);
    assert_eq!(response.status_code, StatusCode::OK);
    assert_eq!(response.body, None);
}
