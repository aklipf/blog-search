use serde::Deserialize;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Node, Window, window};

use crate::search::Article;


#[derive(Deserialize, Debug, Clone)]
pub struct EditConfig{
    id: String,
    html_field: Field,
    article_field: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    template_id: String,
    list_id: String,
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
    #[error("No search exist")]
    NoSearch,
}

#[derive(Clone, Debug)]
pub struct Ui {
    window: Window,
    document: Document,
    config: Config,
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
                .map_err(|_| Error::CannotSetAttribute { attr:attr.clone(), element }),
        }
    }
}

impl Ui {
    pub fn new(config: &Config) -> Self {
        let window = window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        Self { window, document ,config:config.clone()}
    }

    pub fn get_querry(&self) -> Result<String, Error> {
        self.window
            .location()
            .search()
            .map_err(|_| Error::NoSearch)?
            .trim_start_matches('?')
            .split("&")
            .filter_map(|arg| arg.strip_prefix("q="))
            .map(|q| q.to_string())
            .next()
            .ok_or(Error::NoSearch)
    }

    pub fn display(&self, article: &Article) -> Result<(), Error> {
        let list = self
            .document
            .get_element_by_id(self.config.list_id.as_str())
            .ok_or(Error::IdNotFound { id: self.config.template_id.clone() })?;

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
        card.remove_attribute("id");

        for edit in &self.config.edit{
            let value = article.fields.get(&edit.article_field).unwrap();
            card.edit(edit.id.as_str(), &edit.html_field, value)?;
        }

        list.append_child(&card).unwrap();
        Ok(())
    }
}
