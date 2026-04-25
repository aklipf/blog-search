use wasm_bindgen::prelude::*;
use web_sys::console;

mod search;
mod ui;
mod config;

use crate::search::SearchEngin;
use crate::ui::Ui;
use crate::config::Config;

pub fn log(log: String) {
    let array = js_sys::Array::new();
    array.push(&log.into());

    console::log(&array);
}

#[wasm_bindgen]
pub async fn run(config: JsValue) -> Result<(), JsValue> {
    let config: Config=serde_wasm_bindgen::from_value(config)?;

    let ui = Ui::new(&config.ui);

    let search = SearchEngin::new(&config.search).await?;

    let query = ui.get_querry().unwrap();

    let recipes = search.search(query.as_str());
    for recipe in recipes.iter() {
        ui.display(recipe);
    }

    Ok(())
}
