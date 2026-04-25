use wasm_bindgen::prelude::*;
use web_sys::console;

mod config;
mod search;
mod ui;

use crate::config::Config;
use crate::search::SearchEngin;
use crate::ui::Ui;

pub fn log(log: String) {
    let array = js_sys::Array::new();
    array.push(&log.into());

    console::log(&array);
}

#[wasm_bindgen]
pub async fn run(config: JsValue) -> Result<(), JsValue> {
    let config: Config = serde_wasm_bindgen::from_value(config)?;

    let ui = Ui::new(&config.ui);

    let search = match SearchEngin::new(&config.search).await{
        Ok(search)=>search,
        Err(error)=>{
            log(format!("error: {error:?}"));
            panic!("")
        }
    };

    let query = ui.get_querry().unwrap();

    let articles = search.search(query.as_str());
    for article in articles.iter() {
        ui.display(article);
    }

    Ok(())
}
