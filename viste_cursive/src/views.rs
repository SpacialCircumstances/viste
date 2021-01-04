use crate::text::setting;
use cursive::view::{View, ViewWrapper};
use cursive::views::{TextContent, TextView};
use cursive::wrap_impl;
use std::rc::Rc;
use viste_reactive::RWires;

pub trait RView: View {
    type Bindings;

    fn bindings(&self) -> &Self::Bindings;
}

pub struct TextBindings {
    pub content: Rc<RWires<'static, String>>,
}

pub struct RTextView {
    view: TextView,
    bindings: TextBindings,
}

impl RTextView {
    pub fn new() -> Self {
        let tcontent = TextContent::new("");
        let content = setting(tcontent.clone());
        let bindings = TextBindings {
            content: Rc::new(content.into()),
        };
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
