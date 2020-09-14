use crate::elastic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tera::{Context, Tera};
use tracing::info;

#[derive(Serialize, Deserialize)]
struct PackagePage {
    engineers: Value,
    package: String,
}

/// Returns engineers using the package and related package names
pub(crate) async fn html(tera: &Tera, es_url: String, package: String) -> Result<String, ()> {
    // ES search requires it to be lower case
    let package = package.to_lowercase();

    let pkg_page = PackagePage {
        engineers: elastic::search(
            &es_url,
            Some(elastic::add_param(elastic::SEARCH_ENGINEER_BY_PACKAGE, package.clone()).as_str()),
        )
        .await?,
        package,
    };
    //info!("R: {}", refs_page.to_string());

    let html = tera
        .render(
            "package.html",
            &Context::from_value(
                serde_json::to_value(pkg_page).expect("Failed to serialize RefsPage"),
            )
            .expect("Cannot create context"),
        )
        .expect("Cannot render");
    info!("Rendered");

    Ok(html)
}
