use cursive::view::ViewWrapper;
use cursive::views::{TextContent, TextView};
use cursive::{Vec2, View};
use viste_reactive::signals::Node;

pub struct RTextView {
    view: TextView,
    text: Node<'static, String>,
    content: TextContent,
}

impl RTextView {
    pub fn new(text: Node<'static, String>) -> Self {
        let content = text.with_data(|t, _| TextContent::new(t));

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
        self.text.if_changed(|t| self.content.set_content(t));
        self.view.layout(size);
    }
}
