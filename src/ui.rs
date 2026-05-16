use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use wasm_bindgen::prelude::*;
use web_sys::{window, Document, Element, Node, Window};

use crate::search::Article;

#[derive(Deserialize, Debug, Clone)]
pub struct EditConfig {
    id: String,
    html_field: Field,
    article_field: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    page: PageConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PageConfig {
    template_id: String,
    list_id: String,
    inner: Option<Box<Self>>,
    edit: Vec<EditConfig>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("syntax error (id: \"{id}\")")]
    SyntaxError { id: String },
    #[error("id \"{id}\" not found")]
    IdNotFound { id: String },
    #[error("Cannot clone element {element:?}")]
    CannotClone { element: Element },
    #[error("Cannot cast node {node:?} into Element")]
    CannotCast { node: Node },
    #[error("Cannot set attribute {attr} in element {element:?}")]
    CannotSetAttribute { attr: String, element: Element },
}

#[derive(Clone, Debug)]
pub struct Ui {
    window: Window,
    document: Document,
    config: PageConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Field {
    Attribute(String),
    InnerHTML,
}

trait Edit {
    fn edit<ID: AsRef<str>, V: AsRef<str>>(
        &self,
        id: ID,
        field: &Field,
        value: V,
    ) -> Result<(), Error>;
}

impl Edit for web_sys::Element {
    fn edit<ID: AsRef<str>, V: AsRef<str>>(
        &self,
        id: ID,
        field: &Field,
        value: V,
    ) -> Result<(), Error> {
        let result = self.query_selector(format!("#{}", id.as_ref()).as_str());

        let option = result.map_err(|_| Error::SyntaxError {
            id: id.as_ref().to_string(),
        })?;
        let element = option.ok_or(Error::IdNotFound {
            id: id.as_ref().to_string(),
        })?;

        match field {
            Field::InnerHTML => {
                element.set_inner_html(value.as_ref());
                Ok(())
            }
            Field::Attribute(attr) => element
                .set_attribute(attr.as_str(), value.as_ref())
                .map_err(|_| Error::CannotSetAttribute {
                    attr: attr.clone(),
                    element,
                }),
        }
    }
}

impl Ui {
    pub fn new(config: &Config) -> Self {
        let window = window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        Self {
            window,
            document,
            config: config.page.clone(),
        }
    }

    pub fn get_filters_html(&self) -> HashMap<String, HashSet<String>> {
        let mut filters: HashMap<String, HashSet<String>> = HashMap::new();

        if let Ok(nodes) = self
            .document
            .query_selector_all("input[type=\"checkbox\"]:checked")
        {
            for i in 0..nodes.length() {
                if let Some(element) = nodes.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                    let taxonomy = element.get_attribute("name");
                    let term = element.get_attribute("value");
                    if let (Some(tax), Some(term)) = (taxonomy, term) {
                        filters.entry(tax).or_default().insert(term);
                    }
                }
            }
        }

        filters
    }

    pub fn set_filters_html(&self, filters: &HashMap<String, HashSet<String>>) {
        if let Ok(nodes) = self.document.query_selector_all("input[type=\"checkbox\"]") {
            for i in 0..nodes.length() {
                if let Some(input) = nodes.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                    let _ = input.remove_attribute("checked");
                }
            }
        }

        for (taxonomy, terms) in filters {
            for term in terms {
                let selector = format!(
                    "input[type=\"checkbox\"][name=\"{}\"][value=\"{}\"]",
                    taxonomy, term
                );
                if let Ok(Some(input)) = self.document.query_selector(&selector) {
                    let _ = input.set_attribute("checked", "");
                }
            }
        }
    }

    pub fn reset(&self) {
        if let Some(list) = self.document.get_element_by_id(&self.config.list_id) {
            list.set_inner_html("");
        }
    }

    pub fn set_filters_url(&self, filters: &HashMap<String, HashSet<String>>) {
        let location = self.window.location();
        let pathname = location.pathname().unwrap_or_default();
        let current_query = location
            .search()
            .ok()
            .and_then(|s| {
                let params = web_sys::UrlSearchParams::new_with_str(&s).ok()?;
                params.get("query")
            })
            .unwrap_or_default();

        let mut url = format!("{}?query={}", pathname, current_query);
        for (taxonomy, terms) in filters {
            let terms_str: Vec<&str> = terms.iter().map(|s| s.as_str()).collect();
            url.push_str(&format!("&filter={}:{}", taxonomy, terms_str.join(",")));
        }

        if let Ok(history) = self.window.history() {
            let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
        }
    }

    fn url_search(&self) -> Option<String> {
        self.window.location().search().ok()
    }

    fn parse_filter_param(value: &str, filters: &mut HashMap<String, HashSet<String>>) {
        let Ok(decoded) = js_sys::decode_uri_component(value) else {
            return;
        };
        let value = decoded.as_string().unwrap_or_default();
        let Some((taxonomy, terms)) = value.split_once(':') else {
            return;
        };
        for term in terms.split(',') {
            filters
                .entry(taxonomy.to_string())
                .or_default()
                .insert(term.to_string());
        }
    }

    pub fn get_filters_url(&self) -> HashMap<String, HashSet<String>> {
        let search = match self.url_search() {
            Some(s) => s,
            None => return HashMap::new(),
        };
        let mut filters: HashMap<String, HashSet<String>> = HashMap::new();
        for pair in search
            .trim_start_matches('?')
            .split('&')
            .filter(|s| !s.is_empty())
        {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "filter" {
                    Self::parse_filter_param(value, &mut filters);
                }
            }
        }
        filters
    }

    pub fn display(&self, article: &Article) -> Result<(), Error> {
        let list = self
            .document
            .get_element_by_id(self.config.list_id.as_str())
            .ok_or(Error::IdNotFound {
                id: self.config.template_id.clone(),
            })?;

        let template = self
            .document
            .get_element_by_id(self.config.template_id.as_str())
            .ok_or(Error::IdNotFound {
                id: self.config.list_id.clone(),
            })?;

        let node_template = template
            .clone_node_with_deep(true)
            .map_err(|_| Error::CannotClone { element: template })?;
        let card: Element = node_template
            .dyn_into()
            .map_err(|node| Error::CannotCast { node })?;
        let _ = card.remove_attribute("id");

        for edit in &self.config.edit {
            let value = article.fields.get(&edit.article_field).unwrap();
            card.edit(edit.id.as_str(), &edit.html_field, value)?;
        }

        list.append_child(&card).unwrap();
        Ok(())
    }
}
