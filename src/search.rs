use fuzzy_matcher::skim::{SkimMatcherV2, SkimScoreConfig};
use fuzzy_matcher::FuzzyMatcher;
use js_sys::Promise;
use serde::Deserialize;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Response, Text, Window};

use crate::log;

use std::collections::{HashMap, HashSet};

#[derive(Error, Debug)]
pub enum Error {
    #[error("syntax error {error:?}")]
    SyntaxError { error: serde_xml_rs::Error },
    #[error("js error {error:?}")]
    JsError { error: JsValue },
}

#[derive(Deserialize, Debug)]
pub struct Config {
    feed_url: String,
    match_fields: Vec<String>,
    #[serde(default)]
    skim: SkimConfig,
}

#[derive(Deserialize, Debug)]
pub struct SkimConfig {
    #[serde(default)]
    score_match: Option<i32>,
    #[serde(default)]
    gap_start: Option<i32>,
    #[serde(default)]
    gap_extension: Option<i32>,
    #[serde(default)]
    bonus_first_char_multiplier: Option<i32>,
    #[serde(default)]
    bonus_head: Option<i32>,
    #[serde(default)]
    bonus_break: Option<i32>,
    #[serde(default)]
    bonus_camel: Option<i32>,
    #[serde(default)]
    bonus_consecutive: Option<i32>,
    #[serde(default)]
    penalty_case_mismatch: Option<i32>,
}

impl Default for SkimConfig {
    fn default() -> Self {
        Self {
            score_match: None,
            gap_start: None,
            gap_extension: None,
            bonus_first_char_multiplier: None,
            bonus_head: None,
            bonus_break: None,
            bonus_camel: None,
            bonus_consecutive: None,
            penalty_case_mismatch: None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Search {
    taxonomies: Taxonomies,
    pages: Pages,
}

#[derive(Deserialize, Debug)]
pub struct Taxonomies {
    item: Vec<TaxonomyCategory>,
}

#[derive(Deserialize, Debug)]
pub struct TaxonomyCategory {
    name: String,
    terms: TermEntries,
}

#[derive(Deserialize, Debug)]
pub struct TermEntries {
    item: Vec<TermEntry>,
}

#[derive(Deserialize, Debug)]
pub struct TermEntry {
    name: String,
    #[serde(default)]
    link: String,
}

#[derive(Deserialize, Debug)]
pub struct Pages {
    item: Vec<Article>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Article {
    pub taxonomies: HashMap<String, Items>,
    pub fields: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Items {
    #[serde(default)]
    item: Vec<String>,
}

pub struct SearchEngin {
    index: Search,
    match_fields: Vec<String>,
    matcher: SkimMatcherV2,
}

impl Article {
    fn filters(&self) -> HashMap<String, HashSet<String>> {
        self.taxonomies
            .iter()
            .map(|(k, v)| (k.clone(), v.item.iter().cloned().collect()))
            .collect()
    }

    fn has_all_filters(&self, filters: &HashMap<String, HashSet<String>>) -> bool {
        let article_filters = self.filters();

        filters.keys().all(|key| {
            article_filters
                .get(key)
                .zip(filters.get(key))
                .map_or(false, |(a, f)| a.intersection(f).next().is_some())
        })
    }
}

impl SearchEngin {
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let window = window().expect("no global `window` exists");
        let index = Self::load(&window, config.feed_url.as_str()).await?;
        let def = SkimScoreConfig::default();
        let skim_config = SkimScoreConfig {
            score_match: config.skim.score_match.unwrap_or(def.score_match),
            gap_start: config.skim.gap_start.unwrap_or(def.gap_start),
            gap_extension: config.skim.gap_extension.unwrap_or(def.gap_extension),
            bonus_first_char_multiplier: config
                .skim
                .bonus_first_char_multiplier
                .unwrap_or(def.bonus_first_char_multiplier),
            bonus_head: config.skim.bonus_head.unwrap_or(def.bonus_head),
            bonus_break: config.skim.bonus_break.unwrap_or(def.bonus_break),
            bonus_camel: config.skim.bonus_camel.unwrap_or(def.bonus_camel),
            bonus_consecutive: config
                .skim
                .bonus_consecutive
                .unwrap_or(def.bonus_consecutive),
            penalty_case_mismatch: config
                .skim
                .penalty_case_mismatch
                .unwrap_or(def.penalty_case_mismatch),
        };
        log(format!(
            "SkimScoreConfig {{ score_match: {}, gap_start: {}, gap_extension: {}, bonus_first_char_multiplier: {}, bonus_head: {}, bonus_break: {}, bonus_camel: {}, bonus_consecutive: {}, penalty_case_mismatch: {} }}",
            skim_config.score_match,
            skim_config.gap_start,
            skim_config.gap_extension,
            skim_config.bonus_first_char_multiplier,
            skim_config.bonus_head,
            skim_config.bonus_break,
            skim_config.bonus_camel,
            skim_config.bonus_consecutive,
            skim_config.penalty_case_mismatch,
        ));
        log(format!("{index:#?}"));
        let engin = SearchEngin {
            index,
            match_fields: config.match_fields.clone(),
            matcher: SkimMatcherV2::default()
                .ignore_case()
                .score_config(skim_config),
        };

        Ok(engin)
    }

    async fn load(window: &Window, file: &str) -> Result<Search, Error> {
        let future: JsFuture = window.fetch_with_str(file).into();
        let resp: Response = future
            .await
            .map_err(|error| Error::JsError { error })?
            .dyn_into()
            .map_err(|error| Error::JsError { error })?;

        let promise: Promise = resp.text().map_err(|error| Error::JsError { error })?;
        let text: Text = JsFuture::from(promise)
            .await
            .map_err(|error| Error::JsError { error })?
            .into();

        let index: Search = serde_xml_rs::from_str(&text.as_string().unwrap_or_default())
            .map_err(|error| Error::SyntaxError { error })?;
        Ok(index)
    }

    pub fn tokenize(&self, query: &str) -> Vec<String> {
        query.split_whitespace().map(String::from).collect()
    }

    fn fuzzy_search(&self, keyword: &str, keywords: &HashSet<String>) -> Vec<String> {
        let mut matched: Vec<String> = Default::default();

        for choice in keywords.iter() {
            if self
                .matcher
                .fuzzy_match(choice, keyword)
                .is_some_and(|s| s > 0)
            {
                matched.push(choice.clone());
            }
        }

        matched
    }

    pub fn detect_filters(
        &self,
        tokens: &[String],
    ) -> (HashMap<String, HashSet<String>>, Vec<String>) {
        let known: HashMap<String, HashSet<String>> = self
            .index
            .taxonomies
            .item
            .iter()
            .map(|c| {
                let names = c.terms.item.iter().map(|t| t.name.clone()).collect();
                (c.name.clone(), names)
            })
            .collect();

        let mut filters: HashMap<String, HashSet<String>> = HashMap::new();
        let mut remaining: Vec<String> = Vec::new();

        for keyword in tokens {
            let mut matched = false;
            for (key, taxonomy) in &known {
                for token in self.fuzzy_search(keyword, taxonomy) {
                    filters.entry(key.clone()).or_default().insert(token);
                    matched = true;
                }
            }
            if !matched {
                remaining.push(keyword.clone());
            }
        }

        (filters, remaining)
    }

    fn rank(&self, article: &Article, tokens: &[String]) -> u32 {
        let mut score: u32 = 0;

        for token in tokens {
            for field in &self.match_fields {
                if let Some(match_score) = article
                    .fields
                    .get(field.as_str())
                    .and_then(|value| self.matcher.fuzzy_match(value, token))
                    .filter(|&s| s > 0)
                {
                    let weight = if field == "title" { 10 } else { 1 };
                    score = score.saturating_add((match_score.max(0) as u32 + 1) * weight);
                }
            }
        }

        score
    }

    pub fn search(
        &self,
        tokens: &[String],
        filters: &HashMap<String, HashSet<String>>,
    ) -> Vec<Article> {
        let mut articles: Vec<(u32, Article)> = Default::default();

        for article in self.index.pages.item.iter() {
            if article.has_all_filters(filters) {
                let rank = self.rank(article, tokens);
                log(format!(
                    "rank {rank}: {}",
                    article.fields.get("title").unwrap()
                ));
                if tokens.is_empty() || rank > 0 {
                    articles.push((rank, article.clone()));
                }
            }
        }

        articles.sort_by(|a, b| b.0.cmp(&a.0));
        articles.into_iter().map(|(_, a)| a).collect()
    }
}
