use crate::{html, Error};
use lambda::Context;
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

//pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
    //info!("Event: {}", event);
    //info!("Context: {:?}", ctx);

    // get the path from the request
    let raw_path = event["rawPath"].as_str().unwrap_or_default().to_string();

    match raw_path.as_str() {
        "/favicon.ico" => {
            return Ok(serde_json::to_value(ApiGatewayResponse {
                is_base64_encoded: false,
                status_code: 404,
                headers: HashMap::new(),
                body: "".into(),
            })
            .expect("Failed to serialize response"));
        }
        _ => { // do nothing
        }
    }

    let es_url = std::env::var("STACK_MUNCHER_ES_URL").expect("Missing STACK_MUNCHER_ES_URL");

    let tera = match Tera::new("templates/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            panic!();
        }
    };

    // do something useful here
    let html = html::html(&tera, es_url, raw_path)
        .await
        .expect("html() failed");

    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), "text/html".to_owned());

    let resp = ApiGatewayResponse {
        is_base64_encoded: false,
        status_code: 200,
        headers,
        body: html,
    };

    let resp = serde_json::to_value(resp).expect("Failed to serialize response");

    // return back the result
    Ok(resp)
}
