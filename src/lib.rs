use std::collections::{HashMap, HashSet};
use std::panic;
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

pub fn error(error: String) {
    let array = js_sys::Array::new();
    array.push(&error.into());

    console::error(&array);
}

#[wasm_bindgen]
pub async fn load(config: JsValue) -> SearchEngine {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    let config: Config = serde_wasm_bindgen::from_value(config).unwrap();
    let inner = search::SearchEngin::new(&config.search).await.unwrap();
    let ui = Ui::new(&config.ui);

    SearchEngine { inner, ui }
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
            self.ui.display(article).unwrap();
        }
        serde_wasm_bindgen::to_value(&articles).unwrap()
    }

    pub fn search_with_filters(&self, query: &str, filters_js: JsValue) -> JsValue {
        let tokens = self.inner.tokenize(query);
        let filters: HashMap<String, HashSet<String>> =
            serde_wasm_bindgen::from_value(filters_js).unwrap_or_default();
        let articles = self.inner.search(&tokens, &filters);
        for article in articles.iter() {
            log(format!("article {article:#?}"));
            self.ui.display(article).unwrap();
        }
        serde_wasm_bindgen::to_value(&articles).unwrap()
    }

    pub fn get_filters_html(&self) -> JsValue {
        let filters = self.ui.get_filters_html();
        serde_wasm_bindgen::to_value(&filters).unwrap()
    }

    pub fn set_filters_html(&self, filters_js: JsValue) {
        let filters: HashMap<String, HashSet<String>> =
            serde_wasm_bindgen::from_value(filters_js).unwrap_or_default();
        self.ui.set_filters_html(&filters);
    }

    pub fn reset(&self) {
        self.ui.reset();
    }

    pub fn set_filters_url(&self, filters_js: JsValue) {
        let filters: HashMap<String, HashSet<String>> =
            serde_wasm_bindgen::from_value(filters_js).unwrap_or_default();
        self.ui.set_filters_url(&filters);
    }

    pub fn get_filters_url(&self) -> JsValue {
        let filters = self.ui.get_filters_url();
        serde_wasm_bindgen::to_value(&filters).unwrap_or(JsValue::UNDEFINED)
    }
}
