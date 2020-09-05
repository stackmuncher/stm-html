use crate::{html, Error};
use lambda::Context;
use rust_embed::RustEmbed;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tera::Tera;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayResponse {
    // #[serde(skip_serializing_if = "Option::is_none")]
    // cookies: Option<Vec<String>>,
    is_base64_encoded: bool,
    status_code: i32,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(RustEmbed)]
#[folder = "templates"]
struct Asset;

//pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
    //info!("Event: {}", event);
    //info!("Context: {:?}", ctx);

    // get the path from the request
    let raw_path = event["rawPath"].as_str().unwrap_or_default().to_string();

    let es_url = std::env::var("STACK_MUNCHER_ES_URL").expect("Missing STACK_MUNCHER_ES_URL");

    let tera = tera_init()?;

    // do something useful here
    let html = html::html(&tera, es_url, raw_path.clone())
        .await
        .expect("html() failed");

    // an empty response = validation problems -> return 404
    if html.is_empty() {
        let html = html::error_404::html(&tera, raw_path)
            .await
            .expect("Failed to produce 404 page");
        return gw_response(html, 404);
    }

    // return back the result
    gw_response(html, 200)
}

/// Prepares the response with the status and HTML body. May fail and return an error.
fn gw_response(body: String, status_code: i32) -> Result<Value, Error> {
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), "text/html".to_owned());

    let resp = ApiGatewayResponse {
        is_base64_encoded: false,
        status_code,
        headers,
        body,
    };

    Ok(serde_json::to_value(resp).expect("Failed to serialize response"))
}

/// Init Tera instance and load all HTML templates either from the file system
/// (debug) or the binary (release).
fn tera_init() -> Result<Tera, Error> {
    let mut tera = Tera::default();

    // loads the files from the fs or embedded strings
    // see https://github.com/pyros2097/rust-embed
    for file in Asset::iter() {
        let file: &str = &file;
        let content = Asset::get(file).expect("Cannot de-asset HTML");
        let content = std::str::from_utf8(content.as_ref()).expect("Cannot convert HTML for str");

        tera.add_raw_template(file, content)
            .expect("Cannot add raw template");
    }

    Ok(tera)
}
