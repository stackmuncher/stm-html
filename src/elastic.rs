use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use serde_json::Value;
use tracing::error;

pub const USER_IDX: &str = "users";

//pub const SEARCH_TOP_KEYWORDS: &str = r#"{"size":0,"aggs":{"refs":{"terms":{"field":"report.tech.refs_kw.k.keyword","exclude": ["System","TargetFramework","Microsoft","Text","0","1","2"],"size":100},"aggs":{"total":{"sum":{"field":"report.tech.refs_kw.c"}},"sort":{"bucket_sort":{"sort":["_key"]}}}}}}"#;
pub const SEARCH_TOTAL_HIREABLE: &str =
    r#"{"size":0,"aggregations":{"total_hireable":{"terms":{"field":"hireable"}}}}"#;
pub const SEARCH_TOP_USERS: &str =
    r#"{"size":24,"query":{"match_all":{}},"sort":[{"report.timestamp":{"order":"desc"}}]}"#;
pub const SEARCH_TOTAL_REPORTS: &str = r#"{"size":0,"aggs":{"total_reports":{"value_count":{"field":"report.reports_included.keyword"}}}}"#;
pub const SEARCH_TOTAL_TECHS: &str =
    r#"{"size":0,"aggs":{"stack_size":{"cardinality":{"field":"report.tech.language.keyword"}}}}"#;
pub const SEARCH_ENGINEER_BY_LOGIN: &str = r#"{"query":{"term":{"login.keyword":{"value":"%"}}}}"#;
pub const SEARCH_REFS_BY_KEYWORD: &str = r#"{"size":0,"aggregations":{"refs":{"terms":{"field":"report.tech.refs.k.keyword","size":200,"include":"(.*\\.)?%(\\..*)?"}}}}"#;
pub const SEARCH_ENGINEER_BY_KEYWORD: &str = r#"{"size":24,"query":{"bool":{"filter":[{"term":{"report.tech.refs_kw.k.keyword":"%"}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#;
pub const SEARCH_ENGINEER_BY_PACKAGE: &str = r#"{"size":24,"query":{"bool":{"filter":[{"term":{"report.tech.refs.k.keyword":"%"}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#;

/// Run a search with the provided query
pub(crate) async fn search(es_url: &String, query: &str) -> Result<Value, ()> {
    let transport = match Transport::single_node(es_url) {
        Err(e) => {
            error!("Transport level error: {}", e);
            return Err(());
        }
        Ok(v) => v,
    };
    let es_client = Elasticsearch::new(transport);
    let query: Value = serde_json::from_str(query).expect("Failed to JSONify query");

    let response = match es_client
        .search(SearchParts::Index(&[USER_IDX]))
        .body(query)
        .send()
        .await
    {
        Err(e) => {
            error!("Send error: {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    if !response.status_code().is_success() {
        error!(
            "ES QUERY failed. {}",
            response.error_for_status_code_ref().unwrap_err()
        );

        // log a more detailed error message
        if let Ok(r) = response.text().await {
            error!("{}", r);
        }

        return Err(());
    }

    let resp = response
        .text()
        .await
        .expect("Failed to got ES response body");
    let resp: Value = serde_json::from_str(&resp).expect("Failed to serialize ES response body");

    Ok(resp)
}

/// Count number of docs in the index
pub(crate) async fn count(es_url: &String) -> Result<Value, ()> {
    let transport = match Transport::single_node(es_url) {
        Err(e) => {
            error!("Transport level error: {}", e);
            return Err(());
        }
        Ok(v) => v,
    };
    let es_client = Elasticsearch::new(transport);

    let response = match es_client.count(CountParts::Index(&[USER_IDX])).send().await {
        Err(e) => {
            error!("Send error: {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    if !response.status_code().is_success() {
        error!(
            "ES QUERY failed. {}",
            response.error_for_status_code_ref().unwrap_err()
        );

        // log a more detailed error message
        if let Ok(r) = response.text().await {
            error!("{}", r);
        }

        return Err(());
    }

    let resp = response
        .text()
        .await
        .expect("Failed to got ES response body");
    let resp: Value = serde_json::from_str(&resp).expect("Failed to serialize ES response body");

    Ok(resp)
}

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
