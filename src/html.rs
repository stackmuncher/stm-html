use crate::elastic;
use serde::Serialize;
use serde_json::Value;
use tera::{Context, Tera};

#[derive(Serialize)]
struct DataHome {
    total_count: Value,
    hireable_count: Value,
    top_keywords: Value,
    user_list: Value,
}

pub(crate) async fn html(tera: &Tera, es_url: String) -> Result<String, ()> {
    let total_count: Value = elastic::count(&es_url).await?;
    let hireable_count: Value = elastic::search(&es_url, elastic::SEARCH_TOTAL_HIREABLE).await?;
    let top_keywords: Value = elastic::search(&es_url, elastic::SEARCH_TOP_KEYWORDS).await?;
    let user_list: Value = elastic::search(&es_url, elastic::SEARCH_TOP_USERS).await?;

    let data_home = DataHome {
        total_count,
        hireable_count,
        top_keywords,
        user_list,
    };

    let data_home = serde_json::to_value(data_home).expect("Failed to serialize data_home");

    //println!("{}", data_home);
    // panic!();


    let html = tera
        .render(
            "home.html",
            &Context::from_value(data_home).expect("Cannot serialize"),
        )
        .expect("Cannot render");

    Ok(html)
}
