use crate::config::Config;
use crate::elastic;
use html_data::HtmlData;
use regex::Regex;
use tracing::{info, warn};

mod dev;
mod home;
mod html_data;
mod keyword;
// mod package;

const MAX_NUMBER_OF_VALID_SEARCH_TERMS: usize = 4;

/// Routes HTML requests to processing modules. Returns HTML response and TTL value in seconds.
pub(crate) async fn html(
    config: &Config,
    url_path: String,
    url_query: String,
) -> Result<HtmlData, ()> {
    // prepare a common structure for feeding into Tera templates
    let html_data = HtmlData {
        raw_search: url_query.clone(),
        related: None,
        devs: None,
        keywords: Vec::new(),
        langs: Vec::new(),
        keywords_str: None,
        stats: None,
        template_name: "404.html".to_owned(),
        ttl: 600,
        http_resp_code: 404,
    };

    // return 404 for requests that are too long or for some resource related to the static pages
    if url_path.len() > 100 {
        warn!("Invalid request: {}", url_path);
        return Ok(html_data);
    }
    if url_path.starts_with("/about/") || url_path.starts_with("/robots.txt") {
        warn!("Static resource request: {}", url_path);
        return Ok(html_data);
    }

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
            return Ok(html_data);
        }

        // return dev profile page
        return Ok(dev::html(config, login, html_data).await?);
    }

    // is there something in the query string?
    if url_query.len() > 1 {
        // split the query into parts using a few common separators
        let rgx = Regex::new(r#"[#\-\._0-9a-zA-Z]+"#).expect("Wrong search terms regex!");
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

        // will contain values that matches language names
        let mut langs: Vec<String> = Vec::new();
        // will contain the list of keywords to search for
        let mut keywords: Vec<String> = Vec::new();

        // check every search term for what type of a term it is
        for search_term in search_terms {
            // limit the list of valid search terms to 4
            if keywords.len() + langs.len() >= MAX_NUMBER_OF_VALID_SEARCH_TERMS {
                break;
            }
            // searches with a tailing or leading . should be cleaned up
            // it may be possible to have a lead/trail _, maybe
            // I havn't seen a lead/trail - anywhere
            let search_term = search_term.trim_matches('.').trim_matches('-').to_owned();

            // searching for a keyword is different from searching for a fully qualified package name
            // e.g. xml vs System.XML vs SomeVendor.XML
            let (fields, can_be_lang) = if search_term.contains(".") {
                // this is a fully qualified name and cannot be a language
                (
                    vec!["report.tech.refs.k.keyword", "report.tech.pkgs.k.keyword"],
                    false,
                )
            } else {
                // this is a keyword, which may be all there is, but it will be in _kw field anyway
                // this can also be a language
                (
                    vec![
                        "report.tech.language.keyword",
                        "report.tech.refs_kw.k.keyword",
                        "report.tech.pkgs_kw.k.keyword",
                    ],
                    true,
                )
            };

            // get the doc counts for the term
            let counts = elastic::matching_doc_counts(
                &config.es_url,
                &config.dev_idx,
                fields,
                &search_term,
                &config.no_sql_string_invalidation_regex,
            )
            .await?;
            info!("search_term {}: {:?}", search_term, counts);

            // different logic for 2 or 3 field search
            if can_be_lang {
                // this may be a language
                if counts[0] > 0 {
                    langs.push(search_term);
                } else if counts[1] > 0 || counts[2] > 0 {
                    // add it to the list of keywords if there is still room
                    keywords.push(search_term);
                }
            } else if counts[0] > 0 || counts[1] > 0 {
                // .-notation, so can't be a language, but can be a keyword
                // add it to the list of keywords if there is still room
                keywords.push(search_term);
            }
        }

        // run a keyword search
        return Ok(keyword::html(config, keywords, langs, html_data).await?);
    }

    // return the homepage if there is nothing else
    return Ok(home::html(config, html_data).await?);
}
