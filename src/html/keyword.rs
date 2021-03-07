use super::teradata::TeraData;
use crate::config::Config;
use crate::elastic;
use tera::{Context, Tera};
use tracing::info;

/// Returns package names containing the keyword and engineers using them
pub(crate) async fn html(
    tera: &Tera,
    config: &Config,
    keywords: Vec<String>,
    lang: Option<String>,
    tera_data: TeraData,
) -> Result<String, ()> {
    info!("Generating html-keyword");
    info!("KWs: {:?}", keywords);
    info!("Lang: {:?}", lang);

    let devs = elastic::matching_devs(
        &config.es_url,
        &config.dev_idx,
        keywords.clone(),
        lang.clone(),
        &config.no_sql_string_invalidation_regex,
    )
    .await?;

    // pre-build search terms as a string for simplified presentation
    // it should present them all as a list, but for now it uses a simple string
    let mut combined_search_terms = keywords.clone();
    if let Some(v) = lang.clone() {
        combined_search_terms.insert(0, v);
    }
    let combined_search_terms = combined_search_terms.join(" ");

    // put everything together for Tera
    let tera_data = TeraData {
        devs: Some(devs),
        keywords,
        lang: lang,
        keywords_str: Some(combined_search_terms),
        ..tera_data
    };

    let html = tera
        .render(
            "keyword.html",
            &Context::from_value(
                serde_json::to_value(tera_data).expect("Failed to serialize SearchResults"),
            )
            .expect("Cannot create context"),
        )
        .expect("Cannot render");
    info!("Rendered");

    Ok(html)
}
