use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde::Deserialize;
use js_sys::{Object, Array};
use web_sys::{console, window, FetchEvent, Response, ReadableStreamDefaultReader, Text};

#[derive(Deserialize, Debug)]
struct Index {
    search: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "search")]
struct Search {
    item: Vec<Recipe>
}

#[derive(Deserialize, Debug)]
struct Recipe {
    title: String,
    author: String,
    img: String,
    link: String,
    tags: Items,
    ingredients: Items,
    seasons: Items,
    tools: Items,
}

#[derive(Deserialize, Debug)]
struct Items{
    #[serde(default)] 
    item: Vec<String>
}



#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue>{
    // Get the window object
    let window = window().expect("no global `window` exists");

    // Access the document object
    let document = window.document().expect("should have a document on window");

    // Find an element by ID
    let element = document
        .get_element_by_id("my-element")
        .expect("should have element with ID");

    // Modify the element's content
    element.set_inner_html("Hello from Rust!");

    
    let resp: Response = JsFuture::from(
        window.fetch_with_str("./search.xml")
    ).await?.dyn_into()?;
    
    let text = JsFuture::from(resp.text()?).await?;

    element.set_inner_html(&text.as_string().unwrap());
    

    let user: Search = serde_xml_rs::from_str(&text.as_string().unwrap()).expect("Failed to deserialize");
    
    let mut text="".to_string();
    for recipe in user.item{
        text += &recipe.title;
        text += "<br>";
        text += &recipe.author;
        text += "<br>";
        text += &recipe.link;
        text += "<br>";
        text += &recipe.img;
        text += "<br>";
        text += &format!("{:#?}",recipe.tags.item);
        text += "<br>";
        text += &format!("{:#?}",recipe.ingredients.item);
        text += "<br>";
    }
    element.set_inner_html(&text);

    Ok(())
}
