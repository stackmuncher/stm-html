use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A common data format fed to Tera templates
#[derive(Serialize)]
pub(crate) struct TeraData {
    /// System stats
    pub stats: Option<Stats>,
    /// Raw ES response with dev idx docs
    pub devs: Option<Value>,
    /// List of related keywords from  
    pub related: Option<Vec<RelatedKeywords>>,
    /// The raw search string as entered by the user
    pub raw_search: String,
    /// List of keywords extracted from the raw search
    pub keywords: Vec<String>,
    /// A single search terms picked as the language
    pub lang: Option<String>,
    /// Same as `keywords` as a single string
    pub keywords_str: Option<String>,
    /// Name of the HTML template to use. Defaults to 404
    pub template_name: String,
    /// Time to live for the HTTP response
    pub ttl: u32,
    /// HTTP response code
    pub http_resp_code: u32,
}

/// List of related keywords extracted from ES
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RelatedKeywords {
    pub k: String,
    pub c: usize,
}

/// A single entry in the list of stats per metric
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct StatsRecord {
    /// An EPOCH timestamp when the count was taken
    pub ts: i64,
    /// An ISO representation of the moment when the count was taken
    pub iso: String,
    /// The count
    pub c: u64,
}

/// Combined stats. All member arrays are sorted in descending order
#[derive(Serialize, Deserialize)]
pub(crate) struct Stats {
    /// A list of records for repositories
    pub repo: Vec<StatsRecord>,
    /// A list of records for contributors
    pub contributor: Vec<StatsRecord>,
    /// A list of records for devs
    pub dev: Vec<StatsRecord>,
    /// How many technologies are in the stack
    pub stack: Vec<StatsRecord>,
    /// How many devs are available for hire
    pub hireable: Vec<StatsRecord>,
}
