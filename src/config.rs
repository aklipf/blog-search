use crate::search;
use crate::ui::{self, Field};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub search: search::Config,
    pub ui: ui::Config,
}
