use regex::Regex;
use tera::Tera;
use tracing::warn;

mod engineer;
pub(crate) mod error_404;
mod home;
mod keyword;
mod package;

const TTL_HOME: i32 = 600;
const TTL_KW: i32 = 3600;
const TTL_PKG: i32 = 3600;
const TTL_ENGINEER: i32 = 86400;
const TTL_BAD_REQ: i32 = 86400;


pub(crate) const KEYWORD_VALIDATION_REGEX: &str = "[^_0-9a-zA-Z]";

/// Routes HTML requests to processing modules. Returns HTML response and TTL value in seconds.
pub(crate) async fn html(
    tera: &Tera,
    es_url: String,
    raw_path: String,
) -> Result<(String, i32), ()> {
    // is the request too long?
    if raw_path.len() > 100 {
        warn!("Very long request: {}", raw_path);
        return Ok(("".to_owned(), TTL_BAD_REQ));
    }

    // is it a homepage?
    if raw_path.len() < 2 {
        return Ok((home::html(tera, es_url).await?, TTL_HOME));
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
        if kw.len() < 3
            || Regex::new(KEYWORD_VALIDATION_REGEX)
                .expect("Wrong _kw regex!")
                .is_match(&kw)
        {
            warn!("Invalid keyword: {}", raw_path);
            return Ok(("".to_owned(), TTL_BAD_REQ));
        }
        return Ok((keyword::html(tera, es_url, kw).await?, TTL_KW));
    }

    // a single package, e.g. System.Text.Regex
    if raw_path.starts_with("/_pkg/") {
        let pkg = raw_path
            .trim()
            .trim_end_matches("/")
            .trim_start_matches("/_pkg/")
            .trim()
            .to_string();

        // check for dis-allowed chars
        let rgx = Regex::new("[^\\._0-9a-zA-Z]").expect("Wrong _pkg regex!");
        if rgx.is_match(&pkg) {
            warn!("Invalid package: {}", raw_path);
            return Ok(("".to_owned(), TTL_BAD_REQ));
        }
        return Ok((package::html(tera, es_url, pkg).await?, TTL_PKG));
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
        return Ok(("".to_owned(), TTL_BAD_REQ));
    }

    return Ok((engineer::html(tera, es_url, login).await?, TTL_ENGINEER));
}
