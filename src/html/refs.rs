use crate::elastic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tera::{Context, Tera};
//use tracing::info;

#[derive(Serialize, Deserialize)]
struct RefsPage {
    refs: Value,
    engineers: Value,
    keyword: String,
}

/// Returns the default home page
pub(crate) async fn html(tera: &Tera, es_url: String, keyword: String) -> Result<String, ()> {

    // ES search requires it to be lower case
    let keyword = keyword.to_lowercase();

    let refs_page = RefsPage {
        refs: elastic::search(
            &es_url,
            elastic::add_param(elastic::SEARCH_REFS_BY_KEYWORD, keyword.clone()).as_str(),
        )
        .await?,
        engineers: elastic::search(
            &es_url,
            elastic::add_param(elastic::SEARCH_ENGINEER_BY_KEYWORD, keyword.clone()).as_str(),
        )
        .await?,
        keyword: keyword,
    };
    //info!("R: {}", refs_page.to_string());

    let html = tera
        .render(
            "refs.html",
            &Context::from_value(
                serde_json::to_value(refs_page).expect("Failed to serialize data_home"),
            )
            .expect("Cannot serialize"),
        )
        .expect("Cannot render");

    Ok(html)
}
