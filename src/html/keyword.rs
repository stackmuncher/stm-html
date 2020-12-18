use crate::elastic;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tera::{Context, Tera};
use tracing::info;

#[derive(Serialize, Deserialize)]
struct RefsPage {
    refs: Value,
    pkgs: Value,
    engineers: Value,
    keyword: String,
}

/// Returns package names containing the keyword and engineers using them
pub(crate) async fn html(tera: &Tera, es_url: String, keyword: String) -> Result<String, ()> {
    // ES search requires it to be lower case
    let keyword = keyword.to_lowercase();

    let refs_query = elastic::add_param(elastic::SEARCH_REFS_BY_KEYWORD, keyword.clone());
    let pkgs_query = elastic::add_param(elastic::SEARCH_PKGS_BY_KEYWORD, keyword.clone());
    let engineers_query = elastic::add_param(elastic::SEARCH_ENGINEER_BY_KEYWORD, keyword.clone());

    // prepare ES tasks
    let refs = elastic::search(&es_url, Some(&refs_query));
    let pkgs = elastic::search(&es_url, Some(&pkgs_query));
    let engineers = elastic::search(&es_url, Some(&engineers_query));

    // execute all searches in parallel
    let futures = vec![refs, pkgs, engineers];
    let mut resp = join_all(futures).await;

    // restore the results from the response vector removing them one by one
    let refs = resp.remove(0)?;
    let pkgs = resp.remove(0)?;
    let engineers = resp.remove(0)?;

    let refs_page = RefsPage {
        refs,
        pkgs,
        engineers,
        keyword: keyword,
    };
    //info!("R: {}", refs_page.to_string());

    let html = tera
        .render(
            "keyword.html",
            &Context::from_value(
                serde_json::to_value(refs_page).expect("Failed to serialize RefsPage"),
            )
            .expect("Cannot create context"),
        )
        .expect("Cannot render");
    info!("Rendered");

    Ok(html)
}
