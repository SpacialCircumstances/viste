use crate::text::setting;
use cursive::view::{View, ViewWrapper};
use cursive::views::{TextContent, TextView};
use cursive::wrap_impl;
use viste_reactive::RWire;

pub trait RView: View {
    type Bindings;

    fn bindings(&self) -> &Self::Bindings;
}

pub struct TextBindings {
    pub content: RWire<'static, String>,
}

pub struct RTextView {
    view: TextView,
    bindings: TextBindings,
}

impl RTextView {
    pub fn new() -> Self {
        let tcontent = TextContent::new("");
        let content = setting(tcontent.clone());
        let bindings = TextBindings { content };
        Self {
            view: TextView::new_with_content(tcontent),
            bindings,
        }
    }
}

impl Default for RTextView {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewWrapper for RTextView {
    wrap_impl!(self.view: TextView);
}

impl RView for RTextView {
    type Bindings = TextBindings;

    fn bindings(&self) -> &Self::Bindings {
        &self.bindings
    }
}
