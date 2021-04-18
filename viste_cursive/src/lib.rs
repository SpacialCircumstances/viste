use cursive::view::ViewWrapper;
use cursive::views::{TextContent, TextView};
use cursive::{Vec2, View};
use std::rc::Rc;
use viste_reactive::signals::{ReaderToken, Signal};

pub struct RTextView {
    view: TextView,
    text: Signal<'static, Rc<String>>,
    content: TextContent,
}

impl RTextView {
    pub fn new(text: Signal<'static, Rc<String>>) -> Self {
        let content = TextContent::new(&*text.compute(ReaderToken::default()));

        let view = TextView::new_with_content(content.clone());
        Self {
            view,
            text,
            content,
        }
    }
}

impl ViewWrapper for RTextView {
    type V = TextView;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        Some(f(&self.view))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        Some(f(&mut self.view))
    }

    fn wrap_layout(&mut self, size: Vec2) {
        //TODO: Changed?
        self.content
            .set_content(&*self.text.compute(ReaderToken::default()));
        self.view.layout(size);
    }
}
