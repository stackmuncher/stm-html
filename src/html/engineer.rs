use crate::config::Config;
use crate::elastic;
use serde::Serialize;
use serde_json::Value;
use tera::{Context, Tera};
use tracing::info;

#[derive(Serialize)]
struct DataHome {
    total_count: Value,
    hireable_count: Value,
    stack_size: Value,
    reports_count: Value,
    top_keywords: Value,
    user_list: Value,
}

/// Returns the default home page
pub(crate) async fn html(tera: &Tera, config: &Config, login: String) -> Result<String, ()> {
    let query = elastic::add_param(elastic::SEARCH_ENGINEER_BY_LOGIN, login);

    let eng: Value = elastic::search(&config.es_url, &config.dev_idx, Some(query.as_str())).await?;

    //info!("R: {}", eng.to_string());

    let html = tera
        .render(
            "engineer.html",
            &Context::from_value(eng).expect("Cannot serialize"),
        )
        .expect("Cannot render");
    info!("Rendered");

    Ok(html)
}
