use super::teradata::TeraData;
use crate::config::Config;
use crate::elastic;
use tracing::info;

/// Returns the developer profile. Expects a valid login
pub(crate) async fn html(
    config: &Config,
    login: String,
    tera_data: TeraData,
) -> Result<TeraData, ()> {
    info!("Generating html-dev");
    let query = elastic::add_param(
        elastic::SEARCH_ENGINEER_BY_LOGIN,
        login,
        &config.no_sql_string_invalidation_regex,
    );

    let tera_data = TeraData {
        devs: Some(elastic::search(&config.es_url, &config.dev_idx, Some(query.as_str())).await?),
        template_name: "dev.html".to_owned(),
        ttl: 3600,
        http_resp_code: 200,
        ..tera_data
    };

    Ok(tera_data)
}
