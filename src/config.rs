use serde::{Serialize, Deserialize};
use crate::ui::{self,Field};
use crate::search;

pub struct Edit{
    id:String,
    field:Field,
    value:String,
}

#[derive(Deserialize)]
pub struct Config {
    pub search:search::Config,
    pub ui:ui::Config,
}
