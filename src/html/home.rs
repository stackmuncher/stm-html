use crate::elastic;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tera::{Context, Tera};
use tracing::warn;

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
}

#[derive(Serialize, Deserialize, Debug)]
struct RefsKw {
    k: String,
    c: usize,
}

/// Returns the default home page
pub(crate) async fn html(tera: &Tera, es_url: String) -> Result<String, ()> {
    let total_count: Value = elastic::count(&es_url).await?;
    let hireable_count: Value = elastic::search(&es_url, elastic::SEARCH_TOTAL_HIREABLE).await?;
    let stack_size: Value = elastic::search(&es_url, elastic::SEARCH_TOTAL_TECHS).await?;
    let reports_count: Value = elastic::search(&es_url, elastic::SEARCH_TOTAL_REPORTS).await?;
    //let top_keywords: Value = elastic::search(&es_url, elastic::SEARCH_TOP_KEYWORDS).await?;
    let engineers: Value = elastic::search(&es_url, elastic::SEARCH_TOP_USERS).await?;
    let top_keywords: Value = serde_json::to_value(extract_keywords(&engineers))
        .expect("Cannot convert HashSet with ref_kw to Value");

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
            let refs_kw = t.refs_kw;
            if refs_kw.is_none() {
                // this may happen if there are relevant files with no meaningful content in the repo
                continue;
            }

            // these are the keywords we are after
            for kw in refs_kw.unwrap() {
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
