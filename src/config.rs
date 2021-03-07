use regex::Regex;

/// Add the name of the ElasticSearch index to that env var
pub const ES_DEV_IDX_ENV: &str = "STM_HTML_ES_DEV_IDX";
/// Add the name of the ElasticSearch index to that env var
pub const ES_REPO_IDX_ENV: &str = "STM_HTML_ES_REPO_IDX";
/// Add the absolute ElasticSearch URL to that env var
pub const ES_URL_ENV: &str = "STM_HTML_ES_URL";

pub struct Config {
    /// Absolute ElasticSearch URL
    pub es_url: String,
    /// Name of `dev` index
    pub dev_idx: String,
    /// Name of `repo` index
    pub repo_idx: String,
    /// No-SQL field value validation regex - the value would be invalid if it's a match
    pub no_sql_string_invalidation_regex: Regex,
}

impl Config {
    pub fn new() -> Self {
        Config {
            es_url: std::env::var(ES_URL_ENV)
                .expect(&format!(
                    "Missing {} env var with ElasticSearch URL",
                    ES_URL_ENV
                ))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            dev_idx: std::env::var(ES_DEV_IDX_ENV)
                .expect(&format!(
                    "Missing {} env var with ES DEV index name",
                    ES_DEV_IDX_ENV
                ))
                .trim()
                .to_string(),
            repo_idx: std::env::var(ES_REPO_IDX_ENV)
                .expect(&format!(
                    "Missing {} env var with ES REPO index name",
                    ES_REPO_IDX_ENV
                ))
                .trim()
                .to_string(),
            no_sql_string_invalidation_regex: Regex::new(r#"[^#\-\._0-9a-zA-Z]"#)
                .expect("Failed to compile no_sql_string_value_regex"),
        }
    }
}
