use crate::{config::Config, html, Error};
use lambda::Context;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tera::Tera;
use tracing::{info, warn};

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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayRequest {
    raw_path: String,
    headers: HashMap<String, String>,
}

#[derive(RustEmbed)]
#[folder = "templates"]
struct Asset;

//pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
    //info!("Event: {}", event);
    //info!("Context: {:?}", ctx);

    let api_request =
        serde_json::from_value::<ApiGatewayRequest>(event).expect("Failed to deser APIGW request");

    // if Authorization env var is present check if it matches Authorization header
    // this is done for basic protection against direct calls to the api bypassing CloudFront
    if let Ok(auth_var) = std::env::var("Authorization") {
        let auth_header = match api_request.headers.get("authorization") {
            Some(v) => v.clone(),
            None => String::new(),
        };

        if auth_var != auth_header {
            warn!("Unauthorized. Header: {}", auth_header);
            return gw_response("Unauthorized".to_owned(), 403, 3600);
        }
    } else {
        #[cfg(debug_assertions)]
        info!("No Authorization env var - all requests are allowed");
    };

    // get ElasticSearch URL and index names from env vars
    let config = Config::new();

    let tera = tera_init()?;

    // do something useful here
    let (html, ttl) = html::html(&tera, &config, api_request.raw_path.clone())
        .await
        .expect("html() failed");

    // an empty response = validation problems -> return 404
    if html.is_empty() {
        let html = html::error_404::html(&tera, api_request.raw_path)
            .await
            .expect("Failed to produce 404 page");
        return gw_response(html, 404, ttl);
    }

    // return back the result
    gw_response(html, 200, ttl)
}

/// Prepares the response with the status and HTML body. May fail and return an error.
fn gw_response(body: String, status_code: i32, ttl: i32) -> Result<Value, Error> {
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), "text/html".to_owned());
    headers.insert(
        "Cache-Control".to_owned(),
        ["max-age=".to_owned(), ttl.to_string()].concat(),
    );

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
