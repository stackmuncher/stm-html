use regex::Regex;
use tera::Tera;
use tracing::warn;

mod engineer;
mod home;
mod refs;

pub(crate) const KEYWORD_VALIDATION_REGEX: &str = "[^_0-9a-zA-Z]";
   


/// Routes HTML requests to processing modules
pub(crate) async fn html(tera: &Tera, es_url: String, raw_path: String) -> Result<String, ()> {
    // is the request too long?
    if raw_path.len() > 100 {
        warn!("Very long request: {}", raw_path);
        return Err(());
    }

    // is it a homepage?
    if raw_path.len() < 2 {
        return Ok(home::html(tera, es_url).await?);
    }

    // a single keyword, e.g. System or Microsoft
    if raw_path.starts_with("/_kw/") {
        let kw = raw_path
            .trim()
            .trim_end_matches("/")
            .trim_start_matches("/_kw/")
            .trim()
            .to_string();

        // check for dis-allowed chars
        let rgx =  Regex::new(KEYWORD_VALIDATION_REGEX).expect("Wrong _kw regex!");
        if rgx.is_match(&kw) {
            warn!("Invalid keyword: {}", raw_path);
            return Err(());
        }
        return Ok(refs::html(tera, es_url, kw).await?);
    }

    // it must be an engineer id, e.g. rimutaka
    let login = raw_path
        .trim()
        .trim_end_matches("/")
        .trim_start_matches("/")
        .trim()
        .to_string();

    // check for dis-allowed chars
    let rgx = Regex::new("[^_\\-0-9a-zA-Z]").expect("Wrong eng id regex!");
    if rgx.is_match(&login) {
        // there is an invalid character - return an error
        warn!("Invalid login: {}", raw_path);
        return Err(());
    }

    return Ok(engineer::html(tera, es_url, login).await?);
}
