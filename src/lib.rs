use std::collections::{HashMap, HashSet};
use wasm_bindgen::prelude::*;
use web_sys::console;

mod config;
mod search;
mod ui;

use crate::config::Config;
use crate::ui::Ui;

pub fn log(log: String) {
    let array = js_sys::Array::new();
    array.push(&log.into());

    console::log(&array);
}

#[wasm_bindgen]
pub async fn load(config: JsValue) -> Result<SearchEngine, JsValue> {
    let config: Config = serde_wasm_bindgen::from_value(config)?;
    let inner = search::SearchEngin::new(&config.search)
        .await
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let ui = Ui::new(&config.ui);

    Ok(SearchEngine { inner, ui })
}

#[wasm_bindgen]
pub struct SearchEngine {
    inner: search::SearchEngin,
    ui: Ui,
}

impl SearchEngine {
    fn search_articles(&self, query: &str) -> Vec<search::Article> {
        let tokens = self.inner.tokenize(query);
        let (filters, rank_tokens) = self.inner.detect_filters(&tokens);
        self.inner.search(&rank_tokens, &filters)
    }
}

#[wasm_bindgen]
impl SearchEngine {
    pub fn search(&self, query: &str) -> JsValue {
        let articles = self.search_articles(query);
        for article in articles.iter() {
            let _ = self.ui.display(article);
        }
        serde_wasm_bindgen::to_value(&articles).unwrap_or(JsValue::UNDEFINED)
    }

    pub fn search_with_filters(&self, query: &str, filters_js: JsValue) -> JsValue {
        let tokens = self.inner.tokenize(query);
        let filters: HashMap<String, HashSet<String>> =
            serde_wasm_bindgen::from_value(filters_js).unwrap_or_default();
        let articles = self.inner.search(&tokens, &filters);
        for article in articles.iter() {
            let _ = self.ui.display(article);
        }
        serde_wasm_bindgen::to_value(&articles).unwrap_or(JsValue::UNDEFINED)
    }

    pub fn load_filters(&self) -> JsValue {
        let filters = self.ui.load_filters();
        serde_wasm_bindgen::to_value(&filters).unwrap_or(JsValue::UNDEFINED)
    }

    pub fn set_filters(&self, filters_js: JsValue) {
        let filters: Vec<String> = serde_wasm_bindgen::from_value(filters_js).unwrap_or_default();
        log(format!("{filters:#?}"));
        self.ui.set_filters(&filters);
    }

    pub fn reset(&self) {
        self.ui.reset();
    }
}
