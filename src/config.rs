/// Add the name of the ElasticSearch index to that env var
pub const ES_DEV_IDX_ENV: &str = "STM_HTML_ES_DEV_IDX";
/// Add the absolute ElasticSearch URL to that env var
pub const ES_URL_ENV: &str = "STM_HTML_ES_URL";

pub struct Config {
    /// Absolute ElasticSearch URL
    pub es_url: String,
    /// Name of `dev` index
    pub dev_idx: String,
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
        }
    }
}
