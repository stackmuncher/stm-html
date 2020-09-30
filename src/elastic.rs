//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnector;
use rusoto_core::credential::{DefaultCredentialsProvider, ProvideAwsCredentials};
use rusoto_signature::signature::SignedRequest;
use serde_json::Value;
use std::convert::TryInto;
use std::str::FromStr;
use tracing::{debug, error, info};

pub const USER_IDX: &str = "users";

//pub const SEARCH_TOP_KEYWORDS: &str = r#"{"size":0,"aggs":{"refs":{"terms":{"field":"report.tech.refs_kw.k.keyword","exclude": ["System","TargetFramework","Microsoft","Text","0","1","2"],"size":100},"aggs":{"total":{"sum":{"field":"report.tech.refs_kw.c"}},"sort":{"bucket_sort":{"sort":["_key"]}}}}}}"#;
pub const SEARCH_TOTAL_HIREABLE: &str =
    r#"{"size":0,"aggregations":{"total_hireable":{"terms":{"field":"hireable"}}}}"#;
pub const SEARCH_TOP_USERS: &str =
    r#"{"size":24,"query":{"match":{"hireable":{"query":"true"}}},"sort":[{"report.timestamp":{"order":"desc"}}]}"#;
pub const SEARCH_TOTAL_REPORTS: &str = r#"{"size":0,"aggs":{"total_reports":{"value_count":{"field":"report.reports_included.keyword"}}}}"#;
pub const SEARCH_TOTAL_TECHS: &str =
    r#"{"size":0,"aggs":{"stack_size":{"cardinality":{"field":"report.tech.language.keyword"}}}}"#;
pub const SEARCH_ENGINEER_BY_LOGIN: &str = r#"{"query":{"term":{"login.keyword":{"value":"%"}}}}"#;
pub const SEARCH_REFS_BY_KEYWORD: &str = r#"{"size":0,"aggregations":{"refs":{"terms":{"field":"report.tech.refs.k.keyword","size":200,"include":"(.*\\.)?%.*"}}}}"#;
pub const SEARCH_ENGINEER_BY_KEYWORD: &str = r#"{"size":24,"query":{"bool":{"filter":[{"term":{"report.tech.refs_kw.k.keyword":"%"}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#;
pub const SEARCH_ENGINEER_BY_PACKAGE: &str = r#"{"size":24,"query":{"bool":{"filter":[{"term":{"report.tech.refs.k.keyword":"%"}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#;

/// Run a search with the provided query
pub(crate) async fn search(es_url: &String, query: Option<&str>) -> Result<Value, ()> {
    if query.is_some() {
        let es_api_endpoint = [es_url.as_ref(), "/", USER_IDX, "/_search"].concat();
        return call_es_api(es_api_endpoint, Some(query.unwrap().to_string())).await;
    } else {
        let es_api_endpoint = [es_url.as_ref(), "/", USER_IDX, "/_count"].concat();
        return call_es_api(es_api_endpoint, None).await;
    }
}

// /// Count number of docs in the index
// pub(crate) async fn count(es_url: &String) -> Result<Value, ()> {
//     let es_api_endpoint = [es_url.as_ref(), "/", USER_IDX, "/_count"].concat();
//     call_es_api(es_api_endpoint, None).await
// }

/// Inserts a single param in the ES query
pub(crate) fn add_param(query: &str, param: String) -> String {
    let (left, right) = query.split_at(query.find("%").expect("Cannot split the query"));

    [left, param.as_str(), &right[1..]].concat().to_string()
}

#[test]
fn add_param_test() {
    assert_eq!(
        add_param("Hello %!", "world".to_string()),
        "Hello world!".to_string()
    );
}

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
pub(crate) async fn call_es_api(
    es_api_endpoint: String,
    payload: Option<String>,
) -> Result<Value, ()> {
    // prepare METHOD and the payload in one step
    let (method, payload) = match payload {
        None => ("GET", None),
        Some(v) => ("POST", Some(v.as_bytes().to_owned())),
    };
    let payload_id = if payload.is_none() {
        0usize
    } else {
        payload.as_ref().unwrap().len()
    };
    info!("ES query {} started", payload_id);

    // The URL will need to be split into parts to extract region, host, etc.
    let uri = Uri::from_maybe_shared(es_api_endpoint).expect("Invalid ES URL");

    // get the region from teh URL
    let region = uri
        .host()
        .expect("Missing host in ES URL")
        .trim_end_matches(".es.amazonaws.com");
    let (_, region) = region.split_at(region.rfind(".").expect("Invalid ES URL") + 1);
    let region = rusoto_core::Region::from_str(region).expect("Invalid region in the ES URL");

    // prepare the request
    let mut req = SignedRequest::new(method, "es", &region, uri.path());
    req.set_payload(payload);
    req.set_hostname(Some(
        uri.host().expect("Missing host in ES URL").to_string(),
    ));

    // these headers are required by ES
    req.add_header("Content-Type", "application/json");

    // get AWS creds
    let provider = DefaultCredentialsProvider::new().expect("Cannot get default creds provider");
    let credentials = provider.credentials().await.expect("Cannot find creds");

    // sign the request
    req.sign(&credentials);

    // convert the signed request into an HTTP request we can send out
    let req: Request<Body> = req
        .try_into()
        .expect("Cannot convert signed request into hyper request");
    debug!("Http rq: {:?}", req);

    let res = Client::builder()
        .build::<_, hyper::Body>(HttpsConnector::new())
        .request(req)
        .await
        .expect("ES request failed");

    info!("ES query {} response arrived", payload_id);
    let status = res.status();

    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res)
        .await
        .expect("Cannot convert response body to bytes");

    // there should be at least some data returned
    if buf.is_empty() {
        error!("Empty body with status {}", status);
        return Err(());
    }

    // any status other than 200 is an error
    if !status.is_success() {
        error!("Status {}", status);
        log_http_body(&buf);
        return Err(());
    }

    // all responses should be JSON. If it's not JSON it's an error.
    let output =
        Ok(serde_json::from_slice::<Value>(&buf).expect("Failed to convert ES resp to JSON"));
    info!("ES query {} finished", payload_id);
    output
}

/// Logs the body as error!(), if possible.
pub(crate) fn log_http_body(body_bytes: &hyper::body::Bytes) {
    // log the body as-is if it's not too long
    if body_bytes.len() < 5000 {
        let s = match std::str::from_utf8(&body_bytes).to_owned() {
            Err(_e) => "The body is not UTF-8".to_string(),
            Ok(v) => v.to_string(),
        };
        error!("Response body: {}", s);
    } else {
        error!("Response is too long to log: {}B", body_bytes.len());
    }
}
