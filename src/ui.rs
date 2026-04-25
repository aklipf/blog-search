use thiserror::Error;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Node, Window, window, Element};
use serde::Deserialize;

use crate::search::Recipe;

#[derive(Deserialize, Debug)]
pub struct Config{
    template_id: String,
    list_id:String,
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
}

#[derive(Deserialize)]
pub enum Field {
    Attribute(String),
    InnerHTML,
}

trait Edit {
    fn edit<ID: AsRef<str>, V: AsRef<str>>(
        &self,
        id: ID,
        field: Field,
        value: V,
    ) -> Result<(), Error>;
}

impl Edit for web_sys::Element {
    fn edit<ID: AsRef<str>, V: AsRef<str>>(
        &self,
        id: ID,
        field: Field,
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
            Field::Attribute(attr) => {
                element.set_attribute(attr.as_str(), value.as_ref()).map_err(|_| Error::CannotSetAttribute{attr,element})
            }
        }
    }
}

impl Ui {
    pub fn new(config:&Config) -> Self {
        let window = window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        Self { window, document }
    }
    pub fn get_querry(&self) -> Result<String,Error> {
        self.window
            .location()
            .search()
            .map_err(|_| Error::NoSearch)?
            .trim_start_matches('?')
            .split("&")
            .filter_map(|arg| arg.strip_prefix("q="))
            .map(|q| q.to_string())
            .next().ok_or(Error::NoSearch)
    }

    pub fn display(&self, recipe: &Recipe) -> Result<(), Error> {
        let list = self
            .document
            .get_element_by_id("list")
            .ok_or(Error::IdNotFound { id: "list".into() })?;

        let template = self
            .document
            .get_element_by_id("element")
            .ok_or(Error::IdNotFound { id: "element".into() })?;

        let node_template = template.clone_node_with_deep(true).map_err(|_| Error::CannotClone{element:template})?;
        let card: Element = node_template.dyn_into().map_err(|node| Error::CannotCast{node})?;

        card.edit("title", Field::InnerHTML, &recipe.title)?;

        card.edit("img", Field::Attribute("src".to_string()), &recipe.img)?;
        card.edit("img", Field::Attribute("alt".to_string()), &recipe.title)?;
        card.edit(
            "link_img",
            Field::Attribute("href".to_string()),
            &recipe.link,
        )?;
        card.edit(
            "link_title",
            Field::Attribute("href".to_string()),
            &recipe.link,
        )?;

        card.edit("description", Field::InnerHTML, &recipe.description)?;
        card.edit("date", Field::InnerHTML, &recipe.date)?;
        card.edit("cooking-time", Field::InnerHTML, &recipe.cooking_time)?;

        list.append_child(&card).unwrap();
        Ok(())
    }
}
