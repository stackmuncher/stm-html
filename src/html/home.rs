use crate::config::Config;
use crate::elastic;
use futures::future::join_all;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tera::{Context, Tera};
use tracing::{info, warn};

#[derive(Serialize)]
struct DataHome {
    total_count: Value,
    hireable_count: Value,
    stack_size: Value,
    reports_count: Value,
    top_keywords: Value,
    engineers: Value,
}

#[derive(Deserialize, Debug)]
struct EngListResp {
    hits: EngListHits,
}

#[derive(Deserialize, Debug)]
struct EngListHits {
    hits: Vec<EngHit>,
}

#[derive(Deserialize, Debug)]
struct EngHit {
    #[serde(rename(deserialize = "_source"))]
    source: Option<EngSource>,
}

#[derive(Deserialize, Debug)]
struct EngSource {
    report: Option<Report>,
}

#[derive(Deserialize, Debug)]
struct Report {
    tech: Option<Vec<Tech>>,
}

#[derive(Deserialize, Debug)]
struct Tech {
    refs_kw: Option<Vec<RefsKw>>,
    pkgs_kw: Option<Vec<RefsKw>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RefsKw {
    k: String,
    c: usize,
}

/// Returns the default home page
pub(crate) async fn html(tera: &Tera, config: &Config) -> Result<String, ()> {
    // prepare ES tasks
    let total_count = elastic::search(&config.es_url, &config.dev_idx, None);
    let hireable_count = elastic::search(
        &config.es_url,
        &config.dev_idx,
        Some(elastic::SEARCH_TOTAL_HIREABLE),
    );
    let stack_size = elastic::search(
        &config.es_url,
        &config.dev_idx,
        Some(elastic::SEARCH_TOTAL_TECHS),
    );
    let reports_count = elastic::search(
        &config.es_url,
        &config.dev_idx,
        Some(elastic::SEARCH_TOTAL_REPORTS),
    );
    let engineers = elastic::search(
        &config.es_url,
        &config.dev_idx,
        Some(elastic::SEARCH_TOP_USERS),
    );

    // execute all searches in parallel
    let futures = vec![
        total_count,
        hireable_count,
        stack_size,
        reports_count,
        engineers,
    ];
    let mut resp = join_all(futures).await;

    // restore the results
    let total_count = resp.remove(0)?;
    let hireable_count = resp.remove(0)?;
    let stack_size = resp.remove(0)?;
    let reports_count = resp.remove(0)?;
    let engineers = resp.remove(0)?;
    // note, total_count has a different Fn signature and could not be added to join_all

    let top_keywords: Value = serde_json::to_value(extract_keywords(&engineers))
        .expect("Cannot convert HashSet with ref_kw to Value");
    info!("top_keywords extracted");

    let data_home = DataHome {
        total_count,
        hireable_count,
        stack_size,
        reports_count,
        top_keywords,
        engineers,
    };

    let html = tera
        .render(
            "home.html",
            &Context::from_value(
                serde_json::to_value(data_home).expect("Failed to serialize data_home"),
            )
            .expect("Cannot create context"),
        )
        .expect("Cannot render");

    info!("Rendered");

    Ok(html)
}

/// Extracts ref_kw from all engineers and returns a unique list
fn extract_keywords(engineer_list: &Value) -> Vec<RefsKw> {
    let mut collector: HashMap<String, usize> = HashMap::new();
    let rgx = Regex::new(crate::html::KEYWORD_VALIDATION_REGEX).expect("Wrong _kw regex!");

    // the data we need is buried 10 levels deep - keep unwrapping until we are there
    let e_list_resp = serde_json::from_value::<EngListResp>(engineer_list.clone())
        .expect("Cannot deser Eng List");

    for e_source in e_list_resp.hits.hits {
        if e_source.source.is_none() {
            // this should not happen
            warn!("Empty _source on eng list");
            continue;
        }

        let report = e_source.source.unwrap().report;
        if report.is_none() {
            warn!("Empty report on eng list");
            // this should not happen
            continue;
        }

        let tech = report.unwrap().tech;
        if tech.is_none() {
            // this may happen if the repos have no tech we track
            continue;
        }

        for t in tech.unwrap() {
            // code files like .cs and .rs have references (use ...)
            if let Some(refs_kw) = t.refs_kw {
                for kw in refs_kw {
                    // do not add rubbish ones, but log them for reference
                    if rgx.is_match(&kw.k) {
                        warn!("Invalid keyword: {}", kw.k);
                        continue;
                    }
                    // add the keyword to the list and increment its counter
                    *collector.entry(kw.k).or_insert(kw.c) += kw.c;
                }
            }

            // project level files have packages like .csproj or Cargo.toml
            // it's unlikely to have both, pkgs and refs
            if let Some(refs_kw) = t.pkgs_kw {
                // these are the keywords we are after
                for kw in refs_kw {
                    // do not add rubbish ones, but log them for reference
                    if rgx.is_match(&kw.k) {
                        warn!("Invalid keyword: {}", kw.k);
                        continue;
                    }
                    // add the keyword to the list of increment its counter
                    *collector.entry(kw.k).or_insert(kw.c) += kw.c;
                }
            }
        }
    }

    // convert to a vector of `{k:"", c:""}`
    let mut ref_kws: Vec<RefsKw> = collector
        .iter()
        .map(|(k, c)| RefsKw {
            k: k.clone(),
            c: c.clone(),
        })
        .collect();

    // sort by keyword, case-insensitive
    ref_kws.sort_by(|a, b| a.k.to_lowercase().cmp(&b.k.to_lowercase()));

    ref_kws
}
