use tera::Tera;
use tracing::warn;

mod engineer;
mod home;

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

    // is it a keyword?
    // sorry, Kevin Wartford is missing out on his login :(
    if raw_path.starts_with("/kw/") {
        // keyword search
        unimplemented!();
    }

    // it must be an engineer login name

    // extract the login part
    let login = raw_path
        .trim()
        .trim_end_matches("/")
        .trim_start_matches("/").to_string();

    // check for dis-allowed chars
    for b in login.as_bytes() {
        if b == &45u8 {
            // -
            continue;
        } else if b >= &48u8 && b <= &57u8 {
            // 0-9
            continue;
        } else if b >= &65u8 && b <= &90u8 {
            // A-Z
            continue;
        } else if b >= &97u8 && b <= &122u8 {
            // a-z
            continue;
        }
        // there is an invalid character - return an error
        warn!("Invalid login: {}", raw_path);
        return Err(());
    }

    Ok(engineer::html(tera, es_url, login).await?)


}
