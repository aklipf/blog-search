use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Response, Text, Window, window};
use js_sys::Promise;
use thiserror::Error;

use crate::log;

use std::collections::{HashMap, HashSet};
use std::ops::{BitAnd,BitOr};

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
}

#[derive(Deserialize, Debug)]
pub struct Search {
    taxonomies: Vec<Taxonomies>,
    pages: Vec<Article>,
}

#[derive(Deserialize, Debug)]
pub struct Taxonomy {
    name: String,
    terms: Vec<Terms>,
}

#[derive(Deserialize, Debug)]
pub struct Terms {
    name: String,
    terms: Vec<Term>,
}

#[derive(Deserialize, Debug)]
pub struct Term {
    name: String,
    link: String,
}

#[derive(Deserialize, Debug)]
pub struct Taxonomies {
    item: Vec<Taxonomy>,
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Filters {
    taxonomies: HashMap<String, HashSet<String>>,
}

impl From<HashMap<String, Items>> for Filters {
    fn from(values: HashMap<String, Items>) -> Self {
        Filters {
            taxonomies: values
                .into_iter()
                .map(|(k, v)| (k, v.item.iter().cloned().collect()))
                .collect(),
        }
    }
}

pub struct SearchEngin {
    index: Search,
    filters: Filters,
    match_fields: Vec<String>,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        if self.taxonomies.is_empty() {
            return true;
        }
        for (_, taxonomy) in self.taxonomies.iter() {
            if !taxonomy.is_empty() {
                return false;
            }
        }
        return true;
    }
}

impl BitOr<&Filters> for Filters {
    type Output = Self;

    fn bitor(self, rhs: &Self) -> Self::Output {
        let empty : HashSet<String> = Default::default();
        let self_keys: HashSet<String> = self.taxonomies.keys().cloned().collect();
        let rhs_keys: HashSet<String> = rhs.taxonomies.keys().cloned().collect();
        let keys = self_keys.union(&rhs_keys);
        Self {
            taxonomies: keys
                .into_iter()
                .map(|key| {
                    (
                        key.clone(),
                        self.taxonomies
                            .get(key)
                            .unwrap_or(&empty)
                            .union(
                                rhs.taxonomies
                                    .get(key)
                                    .unwrap_or(&empty),
                            )
                            .cloned()
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl BitAnd<&Filters> for Filters {
    type Output = Self;

    fn bitand(self, rhs: &Self) -> Self::Output {
        let empty : HashSet<String> = Default::default();
        let self_keys: HashSet<String> = self.taxonomies.keys().cloned().collect();
        let rhs_keys: HashSet<String> = rhs.taxonomies.keys().cloned().collect();
        let keys = self_keys.intersection(&rhs_keys);
        Self {
            taxonomies: keys
                .into_iter()
                .map(|key| {
                    (
                        key.clone(),
                        self.taxonomies
                            .get(key)
                            .unwrap_or(&empty)
                            .intersection(
                                rhs.taxonomies
                                    .get(key)
                                    .unwrap_or(&empty),
                            )
                            .cloned()
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl Article {
    pub fn filters(&self) -> Filters {
        self.taxonomies.clone().into()
    }

    fn matched(&self, filters: &Filters) -> bool {
        !(self.filters() & filters).is_empty()
    }
}

impl SearchEngin {
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let window = window().expect("no global `window` exists");
        let index = Self::load(&window, config.feed_url.as_str()).await?;
        let mut engin = SearchEngin {
            index,
            filters: Default::default(),
            match_fields: config.match_fields.clone()
        };
        engin.filters = engin.unify_taxonomies();

        Ok(engin)
    }

    async fn load(window: &Window, file: &str) -> Result<Search, Error> {
        let future :JsFuture = window.fetch_with_str(file).into();
        let resp: Response = future.await.map_err(|error| Error::JsError{error})?
            .dyn_into().map_err(|error| Error::JsError{error})?;

        let promise :Promise= resp.text().map_err(|error| Error::JsError{error})?;
        let text: Text = JsFuture::from(promise).await.map_err(|error| Error::JsError{error})?.into();

        let index: Search =
            serde_xml_rs::from_str(&text.as_string().unwrap_or_default()).map_err(|error| Error::SyntaxError{error})?;
        Ok(index)
    }

    fn unify_taxonomies(&self) -> Filters {
        let mut filters:Filters=Default::default();

        for article in self.index.item.iter() {
            filters = filters | &article.taxonomies.clone().into();
        }

        filters
    }

    fn fuzzy_search(&self, keyword: &str, keywords: &HashSet<String>) -> Vec<String> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let mut matched: Vec<String> = Default::default();

        for pattern in keywords.iter() {
            if matcher.fuzzy_match(keyword, pattern).is_some() {
                matched.push(pattern.clone());
            }
        }

        matched
    }

    pub fn detect_filters(&self, querry: &str) -> Filters {
        let mut filters:Filters=Default::default();

        for keyword in querry.split_whitespace() {
            for (key,taxonomy) in &self.filters.taxonomies{
                for token in self.fuzzy_search(keyword, &taxonomy).into_iter(){
                    if let Some(tax)=filters.taxonomies.get_mut(key){
                        tax.insert(token);
                    }
                    else{
                        filters.taxonomies.insert(key.clone(),[token].into_iter().collect());
                    }
                }
            }
        }

        filters
    }

    pub fn search(&self, query: &str) -> Vec<Article> {
        let matcher = SkimMatcherV2::default().ignore_case();

        let filters = self.detect_filters(query);

        let mut articles: Vec<Article> = Default::default();

        for article in self.index.item.iter() {
            if article.matched(&filters) {
                articles.push(article.clone());
                log(format!("filters: {filters:#?} {}", article.fields.get("title").unwrap()));
                continue;
            }

            for pattern in query.split_whitespace() {
                for field in &self.match_fields{
                    if let Some(value) = article.fields.get(field.as_str()){
                        if matcher.fuzzy_match(value, pattern).is_some(){
                            articles.push(article.clone());
                            log(format!("{field}: {pattern:#?} {}",article.fields.get("title").unwrap()));
                        }
                    }
                }
            }
        }

        articles
    }
}
