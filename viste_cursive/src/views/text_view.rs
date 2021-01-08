use crate::text::{appending, setting};
use cursive::view::ViewWrapper;
use cursive::views::{TextContent, TextView};
use cursive::wrap_impl;
use std::rc::Rc;
use viste_reactive::{RWire, RWires};

pub struct RTextView {
    view: TextView,
}

impl RTextView {
    pub fn new() -> Self {
        let tcontent = TextContent::new("");
        Self {
            view: TextView::new_with_content(tcontent),
        }
    }

    pub fn set<'a>(&mut self) -> RWires<'a, String> {
        setting(self.view.get_shared_content()).into()
    }

    pub fn append<'a>(&mut self) -> RWires<'a, String> {
        appending(self.view.get_shared_content()).into()
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
