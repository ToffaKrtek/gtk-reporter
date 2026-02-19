use gtk::{Paned, Window, prelude::*};

use crate::state::State;

pub struct App {
    pub window: Window,
    pub content: Content,
}
pub struct Content {
    pub container: Paned,
    pub state: State,
}
