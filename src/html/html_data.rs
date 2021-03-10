use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A common data format fed to Tera templates
#[derive(Serialize)]
pub(crate) struct HtmlData {
    /// System stats
    pub stats: Option<Value>,
    /// Raw ES response with dev idx docs
    pub devs: Option<Value>,
    /// List of related keywords from  
    pub related: Option<Vec<RelatedKeywords>>,
    /// The raw search string as entered by the user
    pub raw_search: String,
    /// List of keywords extracted from the raw search
    pub keywords: Vec<String>,
    /// A list of search terms matching known languages
    pub langs: Vec<String>,
    /// Same as `keywords` as a single string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_str: Option<String>,
    /// A normalized version of the user login for dev profile page title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_str: Option<String>,
    /// Name of the HTML template to use. Defaults to 404
    pub template_name: String,
    /// Time to live for the HTTP response
    pub ttl: u32,
    /// HTTP response code
    pub http_resp_code: u32,
    /// Contents of HTML meta-tag for bots (nofollow, noindex), if any
    /// e.g. `<meta name="robots" content="noindex">` for `rust+actix` search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_robots: Option<String>,
}

/// List of related keywords extracted from ES
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RelatedKeywords {
    pub k: String,
    pub c: usize,
}
