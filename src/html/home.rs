use super::teradata::{RelatedKeywords, Stats, StatsRecord, TeraData};
use crate::config::Config;
use crate::elastic;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{info, warn};

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
    refs_kw: Option<Vec<RelatedKeywords>>,
    pkgs_kw: Option<Vec<RelatedKeywords>>,
}

/// Returns the default home page
pub(crate) async fn html(config: &Config, tera_data: TeraData) -> Result<TeraData, ()> {
    info!("Generating html-home");

    // grab a bunch of latest additions and updates to dev idx
    let devs = elastic::search(
        &config.es_url,
        &config.dev_idx,
        Some(elastic::SEARCH_TOP_USERS),
    )
    .await?;

    // get stats
    // just dummy stats for now - not sure what the best way of retrieving them is
    let stats_record = vec![StatsRecord {
        ts: 0,
        iso: String::new(),
        c: 1,
    }];
    let stats = Stats {
        repo: stats_record.clone(),
        contributor: stats_record.clone(),
        dev: stats_record.clone(),
        stack: stats_record.clone(),
        hireable: stats_record.clone(),
    };

    // combine everything together for Tera
    let tera_data = TeraData {
        related: Some(extract_keywords(&devs)),
        devs: Some(devs),
        stats: Some(stats),
        template_name: "home.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        ..tera_data
    };

    Ok(tera_data)
}

/// Extracts ref_kw from all engineers and returns a unique list
fn extract_keywords(engineer_list: &Value) -> Vec<RelatedKeywords> {
    let mut collector: HashMap<String, usize> = HashMap::new();
    let rgx = Regex::new(r#"[^\-_0-9a-zA-Z]"#).expect("Wrong _kw regex!");

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
    let mut ref_kws: Vec<RelatedKeywords> = collector
        .iter()
        .map(|(k, c)| RelatedKeywords {
            k: k.clone(),
            c: c.clone(),
        })
        .collect();

    // sort by keyword, case-insensitive
    ref_kws.sort_by(|a, b| a.k.to_lowercase().cmp(&b.k.to_lowercase()));

    info!("Dev keywords extracted");

    ref_kws
}
