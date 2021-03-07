use crate::config::Config;
use crate::elastic;
use regex::Regex;
use tera::Tera;
use teradata::TeraData;
use tracing::{info, warn};

mod dev;
pub(crate) mod error_404;
mod home;
mod keyword;
mod teradata;
// mod package;

const TTL_HOME: i32 = 600;
const TTL_KW: i32 = 3600;
const TTL_ENGINEER: i32 = 86400;
const TTL_BAD_REQ: i32 = 86400;

pub(crate) const KEYWORD_VALIDATION_REGEX: &str = r#"[^\-_0-9a-zA-Z]"#;

/// Routes HTML requests to processing modules. Returns HTML response and TTL value in seconds.
pub(crate) async fn html(
    tera: &Tera,
    config: &Config,
    url_path: String,
    url_query: String,
) -> Result<(String, i32), ()> {
    // is the request too long or is it for some resource related to the static pages?
    if url_path.len() > 100 || url_path.starts_with("/about/") {
        warn!("Very long request: {}", url_path);
        return Ok(("".to_owned(), TTL_BAD_REQ));
    }

    // prepare a common structure for feeding into Tera templates
    let tera_data = TeraData {
        raw_search: url_query.clone(),
        related: None,
        devs: None,
        keywords: Vec::new(),
        lang: None,
        keywords_str: None,
        stats: None,
    };

    // check if there is a path - it can be the developer login
    // there shouldn't be any other paths at this stage
    if url_path.len() > 1 {
        // it must be a dev login that matches the one on github, e.g. rimutaka
        let login = url_path
            .trim()
            .trim_end_matches("/")
            .trim_start_matches("/")
            .trim()
            .to_string();

        // is it a valid format for a dev login?
        if config.no_sql_string_invalidation_regex.is_match(&login) {
            warn!("Invalid dev login: {}", url_path);
            return Ok(("".to_owned(), TTL_BAD_REQ));
        }

        // return dev profile page
        return Ok((
            dev::html(tera, config, login, tera_data).await?,
            TTL_ENGINEER,
        ));
    }

    // is there something in the query string?
    if url_query.len() > 1 {
        // split the query into parts using a few common separators
        let rgx = Regex::new(r#"[\-\._0-9a-zA-Z]+"#).expect("Wrong search terms regex!");
        let search_terms = rgx
            .find_iter(&url_query)
            .map(|v| v.as_str().to_owned())
            .collect::<Vec<String>>();
        info!("Terms: {:?}", search_terms);

        // normalise and dedupe the search terms
        let mut search_terms = search_terms
            .iter()
            .map(|v| v.to_lowercase())
            .collect::<Vec<String>>();
        search_terms.dedup();
        let search_terms = search_terms;

        // is it a single part that matches a dev name?
        if search_terms.len() == 1 {
            let counts = elastic::matching_doc_counts(
                &config.es_url,
                &config.dev_idx,
                vec![
                    "login.keyword",
                    "report.tech.language.keyword",
                    "report.tech.refs_kw.k.keyword",
                ],
                &search_terms[0],
                &config.no_sql_string_invalidation_regex,
            )
            .await?;

            // return a dev profile if there is a match
            if counts[0] == 1 {
                return Ok((
                    dev::html(tera, config, search_terms[0].clone(), tera_data).await?,
                    TTL_ENGINEER,
                ));
            } else if counts[1] > 0 {
                return Ok((
                    keyword::html(
                        tera,
                        config,
                        Vec::new(),
                        Some(search_terms[0].clone()),
                        tera_data,
                    )
                    .await?,
                    TTL_KW,
                ));
            } else {
                return Ok((
                    keyword::html(tera, config, vec![search_terms[0].clone()], None, tera_data)
                        .await?,
                    TTL_KW,
                ));
            }
        }
        // multipart search
        else {
            // will contain the first value that matches the language
            let mut lang: Option<String> = None;
            // will contain the list of keywords to search for
            let mut keywords: Vec<String> = Vec::new();

            // check every search term for what type of a term it is
            for search_term in search_terms {
                let counts = elastic::matching_doc_counts(
                    &config.es_url,
                    &config.dev_idx,
                    vec![
                        "report.tech.language.keyword",
                        "report.tech.refs_kw.k.keyword",
                        "report.tech.pkgs_kw.k.keyword",
                    ],
                    &search_term,
                    &config.no_sql_string_invalidation_regex,
                )
                .await?;
                info!("search_term {}: {:?}", search_term, counts);
                // is it a language?
                // only assign it once
                if counts[0] > 0 && lang.is_none() {
                    lang = Some(search_term);
                } else if (counts[1] > 0 || counts[2] > 0) && keywords.len() < 3 {
                    // add it to the list of keywords if there is still room
                    keywords.push(search_term);
                }
            }

            // run a keyword search
            return Ok((
                keyword::html(tera, config, keywords, lang, tera_data).await?,
                TTL_KW,
            ));
        }
    }

    // return the homepage if there is nothing else
    return Ok((home::html(tera, config, tera_data).await?, TTL_HOME));
}
