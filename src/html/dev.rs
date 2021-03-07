use super::teradata::TeraData;
use crate::config::Config;
use crate::elastic;
use tera::{Context, Tera};
use tracing::info;

/// Returns the developer profile. Expects a valid login
pub(crate) async fn html(
    tera: &Tera,
    config: &Config,
    login: String,
    tera_data: TeraData,
) -> Result<String, ()> {
    info!("Generating html-dev");
    let query = elastic::add_param(
        elastic::SEARCH_ENGINEER_BY_LOGIN,
        login,
        &config.no_sql_string_invalidation_regex,
    );

    let tera_data = TeraData {
        devs: Some(elastic::search(&config.es_url, &config.dev_idx, Some(query.as_str())).await?),
        ..tera_data
    };

    let html = tera
        .render(
            "dev.html",
            &Context::from_value(
                serde_json::to_value(tera_data).expect("Failed to serialize tera_data"),
            )
            .expect("Cannot serialize"),
        )
        .expect("Cannot render");
    info!("Rendered");

    Ok(html)
}
