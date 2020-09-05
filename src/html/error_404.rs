use serde::Serialize;
use tera::{Context, Tera};
//use tracing::info;

#[derive(Serialize)]
struct Page404 {
    search: String,
}

/// Returns package names containing the keyword and engineers using them
pub(crate) async fn html(tera: &Tera, raw_path: String) -> Result<String, ()> {

    // remove the prefix for readability
    let search = if raw_path.starts_with("/_kw/") {
        raw_path[4..].to_owned()
    } else if raw_path.starts_with("/_pkg/") {
        raw_path[5..].to_owned()
    } else {
        raw_path[..].to_owned()
    };

    let html = tera
        .render(
            "404.html",
            &Context::from_value(
                serde_json::to_value(Page404 { search }).expect("Failed to serialize Page404"),
            )
            .expect("Cannot create context"),
        )
        .expect("Cannot render");

    Ok(html)
}
