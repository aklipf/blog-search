use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Response, Text, Window, window};

use crate::log;

use std::collections::HashSet;
use std::ops::BitAnd;

#[derive(Deserialize, Debug)]
pub struct Config{
    feed_url: String
}

#[derive(Deserialize, Debug)]
#[serde(rename = "search")]
pub struct Search {
    item: Vec<Recipe>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Recipe {
    pub title: String,
    pub author: String,
    pub img: String,
    pub link: String,
    pub description: String,
    pub date: String,
    #[serde(rename = "cooking-time")]
    pub cooking_time: String,
    pub tags: Items,
    pub ingredients: Items,
    pub seasons: Items,
    pub tools: Items,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Items {
    #[serde(default)]
    item: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Filters {
    tags: HashSet<String>,
    ingredients: HashSet<String>,
    seasons: HashSet<String>,
    tools: HashSet<String>,
}

pub struct SearchEngin {
    index: Search,
    filters: Filters,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
            && self.ingredients.is_empty()
            && self.seasons.is_empty()
            && self.tools.is_empty()
    }
}

impl BitAnd<&Filters> for Filters {
    type Output = Self;

    fn bitand(self, rhs: &Self) -> Self::Output {
        Self {
            tags: self.tags.intersection(&rhs.tags).cloned().collect(),
            ingredients: self
                .ingredients
                .intersection(&rhs.ingredients)
                .cloned()
                .collect(),
            seasons: self.seasons.intersection(&rhs.seasons).cloned().collect(),
            tools: self.tools.intersection(&rhs.tools).cloned().collect(),
        }
    }
}

impl Recipe {
    pub fn filters(&self) -> Filters {
        Filters {
            tags: self.tags.item.iter().cloned().collect(),
            ingredients: self.ingredients.item.iter().cloned().collect(),
            seasons: self.seasons.item.iter().cloned().collect(),
            tools: self.tools.item.iter().cloned().collect(),
        }
    }

    fn matched(&self, filters: &Filters) -> bool {
        !(self.filters() & filters).is_empty()
    }
}

impl SearchEngin {
    pub async fn new(config:&Config) -> Result<Self, JsValue> {
        let window = window().expect("no global `window` exists");
        let index = Self::load(&window, config.feed_url.as_str()).await?;
        let mut engin = SearchEngin {
            index,
            filters: Default::default(),
        };
        engin.filters = engin.unify_taxonomies();

        Ok(engin)
    }

    async fn load(window: &Window, file: &str) -> Result<Search, JsValue> {
        let resp: Response = JsFuture::from(window.fetch_with_str(file))
            .await?
            .dyn_into()?;

        let text: Text = JsFuture::from(resp.text()?).await?.into();

        let index: Search =
            serde_xml_rs::from_str(&text.as_string().unwrap()).expect("Failed to deserialize");
        Ok(index)
    }

    fn unify_taxonomies(&self) -> Filters {
        let mut tags: HashSet<String> = Default::default();
        let mut ingredients: HashSet<String> = Default::default();
        let mut seasons: HashSet<String> = Default::default();
        let mut tools: HashSet<String> = Default::default();

        for recipe in self.index.item.iter() {
            for tag in recipe.tags.item.iter() {
                tags.insert(tag.clone());
            }

            for ingredient in recipe.ingredients.item.iter() {
                ingredients.insert(ingredient.clone());
            }

            for season in recipe.seasons.item.iter() {
                seasons.insert(season.clone());
            }

            for tool in recipe.tools.item.iter() {
                tools.insert(tool.clone());
            }
        }

        Filters {
            tags,
            ingredients,
            seasons,
            tools,
        }
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
        let mut tags: HashSet<String> = Default::default();
        let mut ingredients: HashSet<String> = Default::default();
        let mut seasons: HashSet<String> = Default::default();
        let mut tools: HashSet<String> = Default::default();

        for keyword in querry.split_whitespace() {
            for w in self.fuzzy_search(keyword, &self.filters.tags).into_iter() {
                tags.insert(w);
            }
            for w in self
                .fuzzy_search(keyword, &self.filters.ingredients)
                .into_iter()
            {
                ingredients.insert(w);
            }
            for w in self
                .fuzzy_search(keyword, &self.filters.seasons)
                .into_iter()
            {
                seasons.insert(w);
            }
            for w in self.fuzzy_search(keyword, &self.filters.tools).into_iter() {
                tools.insert(w);
            }
        }
        Filters {
            tags,
            ingredients,
            seasons,
            tools,
        }
    }

    pub fn search(&self, querry: &str) -> Vec<Recipe> {
        let matcher = SkimMatcherV2::default().ignore_case();

        let filters = self.detect_filters(querry);

        let mut recipes: Vec<Recipe> = Default::default();

        for recipe in self.index.item.iter() {
            if recipe.matched(&filters) {
                recipes.push(recipe.clone());
                log(format!("filters: {filters:#?} {}", recipe.title));
                continue;
            }

            for pattern in querry.split_whitespace() {
                if matcher
                    .fuzzy_match(recipe.title.as_str(), pattern)
                    .is_some()
                {
                    recipes.push(recipe.clone());
                    log(format!("title: {pattern:#?} {}", recipe.title));
                    break;
                }
                if matcher
                    .fuzzy_match(recipe.author.as_str(), pattern)
                    .is_some()
                {
                    recipes.push(recipe.clone());
                    log(format!("author {pattern:#?} {}", recipe.author));
                    break;
                }
            }
        }

        recipes
    }
}
