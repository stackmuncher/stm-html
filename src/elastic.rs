//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use futures::future::join_all;
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnector;
use regex::Regex;
use rusoto_core::credential::{DefaultCredentialsProvider, ProvideAwsCredentials};
use rusoto_signature::signature::SignedRequest;
use serde::Deserialize;
use serde_json::Value;
use std::convert::TryInto;
use std::str::FromStr;
use tracing::{debug, error, info};

//pub const SEARCH_TOP_KEYWORDS: &str = r#"{"size":0,"aggs":{"refs":{"terms":{"field":"report.tech.refs_kw.k.keyword","exclude": ["System","TargetFramework","Microsoft","Text","0","1","2"],"size":100},"aggs":{"total":{"sum":{"field":"report.tech.refs_kw.c"}},"sort":{"bucket_sort":{"sort":["_key"]}}}}}}"#;
// pub const SEARCH_TOTAL_HIREABLE: &str =
//     r#"{"size":0,"aggregations":{"total_hireable":{"terms":{"field":"hireable"}}}}"#;
pub const SEARCH_TOP_USERS: &str = r#"{"size":24,"query":{"match":{"hireable":{"query":"true"}}},"sort":[{"report.timestamp":{"order":"desc"}}]}"#;
// pub const SEARCH_TOTAL_TECHS: &str =
//     r#"{"size":0,"aggs":{"stack_size":{"cardinality":{"field":"report.tech.language.keyword"}}}}"#;
pub const SEARCH_ENGINEER_BY_LOGIN: &str = r#"{"query":{"term":{"login.keyword":{"value":"%"}}}}"#;
// pub const SEARCH_REFS_BY_KEYWORD: &str = r#"{"size":0,"aggregations":{"refs":{"terms":{"field":"report.tech.refs.k.keyword","size":200,"include":"(.*\\.)?%.*"}}}}"#;
// pub const SEARCH_PKGS_BY_KEYWORD: &str = r#"{"size":0,"aggregations":{"pkgs":{"terms":{"field":"report.tech.pkgs.k.keyword","size":200,"include":"(.*\\.)?%.*"}}}}"#;
// pub const SEARCH_ENGINEER_BY_KEYWORD: &str = r#"{"size":24,"query":{"bool":{"should":[{"term":{"report.tech.pkgs_kw.k.keyword":"%"}},{"term":{"report.tech.refs_kw.k.keyword":"%"}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#;
// pub const SEARCH_ENGINEER_BY_PACKAGE: &str = r#"{"size":24,"query":{"bool":{"should":[{"term":{"report.tech.pkgs.k.keyword":"%"}},{"term":{"report.tech.refs.k.keyword":"%"}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#;

/// Member of ESHitsCount
#[derive(Deserialize)]
struct ESHitsCountTotals {
    value: u64,
}

/// Member of ESHitsCount
#[derive(Deserialize)]
struct ESHitsCountHits {
    total: ESHitsCountTotals,
}

/// Corresponds to ES response metadata
/// ```json
/// {
///     "took" : 652,
///     "timed_out" : false,
///     "_shards" : {
///         "total" : 5,
///         "successful" : 5,
///         "skipped" : 0,
///         "failed" : 0
///     },
///     "hits" : {
///         "total" : {
///         "value" : 0,
///         "relation" : "eq"
///         },
///         "max_score" : null,
///         "hits" : [ ]
///     }
/// }
/// ```
#[derive(Deserialize)]
struct ESHitsCount {
    hits: ESHitsCountHits,
}

/// Run a search with the provided query.
/// * es_url: elastucsearch url
/// * idx: ES index name
/// * query: the query text, if any for *_search* or `None` for *_count*
pub(crate) async fn search(
    es_url: &String,
    idx: &String,
    query: Option<&str>,
) -> Result<Value, ()> {
    if query.is_some() {
        let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_search"].concat();
        return call_es_api(es_api_endpoint, Some(query.unwrap().to_string())).await;
    } else {
        let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_count"].concat();
        return call_es_api(es_api_endpoint, None).await;
    }
}

/// Inserts a single param in the ES query in place of %. The param may be repeated within the query multiple times.
/// Panics if the param is unsafe for no-sql queries.
pub(crate) fn add_param(
    query: &str,
    param: String,
    no_sql_string_invalidation_regex: &Regex,
) -> String {
    // validate the param
    if no_sql_string_invalidation_regex.is_match(&param) {
        panic!("Unsafe param value: {}", param);
    }

    let mut modded_query = query.to_string();

    // loop through the query until there are no more % to replace
    while modded_query.contains("%") {
        let (left, right) =
            modded_query.split_at(modded_query.find("%").expect("Cannot split the query"));

        modded_query = [left, param.as_str(), &right[1..]].concat().to_string();
    }

    modded_query
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

/// Returns the number of ES docs that match the query. The field name is not validated or sanitized.
/// Returns an error if the field value contains anything other than alphanumerics and `.-_`.
pub(crate) async fn matching_doc_count(
    es_url: &String,
    idx: &String,
    field: &str,
    field_value: &String,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<u64, ()> {
    // validate field_value for possible no-sql injection
    if no_sql_string_invalidation_regex.is_match(field_value) {
        error!("Invalid field_value: {}", field_value);
        return Err(());
    }

    // the query must be build inside this fn to get a consistent response
    let query = [
        r#"{"query":{"match":{""#,
        field,
        r#"":""#,
        field_value,
        r#""}},"size":0}"#,
    ]
    .concat();

    let es_api_endpoint = [
        es_url.as_ref(),
        "/",
        idx,
        "/_search?filter_path=aggregations.total.buckets",
    ]
    .concat();
    let count = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    // extract the actual value from a struct like this
    // {
    //     "took" : 652,
    //     "timed_out" : false,
    //     "_shards" : {
    //       "total" : 5,
    //       "successful" : 5,
    //       "skipped" : 0,
    //       "failed" : 0
    //     },
    //     "hits" : {
    //       "total" : {
    //         "value" : 0,
    //         "relation" : "eq"
    //       },
    //       "max_score" : null,
    //       "hits" : [ ]
    //     }
    // }
    let count = match serde_json::from_value::<ESHitsCount>(count) {
        Ok(v) => v.hits.total.value,
        Err(e) => {
            error!(
                "Failed to doc count response for idx:{}, field: {}, value: {} with {}",
                idx, field, field_value, e
            );
            return Err(());
        }
    };

    Ok(count)
}

/// Executes multiple doc counts queries in parallel and returns the results in the same order.
/// Returns an error if any of the queries fail.
pub(crate) async fn matching_doc_counts(
    es_url: &String,
    idx: &String,
    fields: Vec<&str>,
    field_value: &String,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Vec<u64>, ()> {
    let mut futures: Vec<_> = Vec::new();

    for field in fields {
        futures.push(matching_doc_count(
            es_url,
            idx,
            field,
            field_value,
            no_sql_string_invalidation_regex,
        ));
    }

    // execute all searches in parallel and unwrap the results
    let mut counts: Vec<u64> = Vec::new();
    for count in join_all(futures).await {
        match count {
            Err(_) => {
                return Err(());
            }
            Ok(v) => {
                counts.push(v);
            }
        }
    }

    Ok(counts)
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

/// Returns up to 24 matching docs from DEV idx depending on the params. The query is built to match the list of params.
/// Lang and KW params are checked for No-SQL injection.
pub(crate) async fn matching_devs(
    es_url: &String,
    dev_idx: &String,
    keywords: Vec<String>,
    langs: Vec<String>,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Value, ()> {
    // sample query
    // {"size":24,"track_scores":true,"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}

    // a collector of must clauses
    let mut must_clauses: Vec<String> = Vec::new();

    // build language clause
    for lang in langs {
        // validate field_value for possible no-sql injection
        if no_sql_string_invalidation_regex.is_match(&lang) {
            error!("Invalid lang: {}", lang);
            return Err(());
        }

        // language clause is different from keywords clause
        let clause = [
            r#"{"match":{"report.tech.language.keyword":""#,
            &lang,
            r#""}}"#,
        ]
        .concat();

        must_clauses.push(clause);
    }

    // build keywords clauses
    for keyword in keywords {
        // validate field_value for possible no-sql injection
        if no_sql_string_invalidation_regex.is_match(&keyword) {
            error!("Invalid keyword: {}", keyword);
            return Err(());
        }

        // query  pkgs and refs if the name is qualified or pkgs_kw and refs_kw if it's not
        let qual_unqual_clause = if keyword.contains(".") {
            r#"","fields":["report.tech.pkgs.k.keyword","report.tech.refs.k.keyword"]}}"#
        } else {
            r#"","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}"#
        };

        // using multimatch because different techs have keywords in different places
        let clause = [r#"{"multi_match":{"query":""#, &keyword, qual_unqual_clause].concat();

        must_clauses.push(clause);
    }

    // combine the clauses
    let clauses = must_clauses.join(",");

    // combine everything into a single query
    let query = [
        r#"{"size":24,"track_scores":true,"query":{"bool":{"must":["#,
        &clauses,
        r#"]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#,
    ]
    .concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_search"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    Ok(es_response)
}
