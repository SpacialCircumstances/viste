use cursive::views::{Dialog, TextView};
use viste_cursive::RTextView;
use viste_reactive::signals::World;

fn main() {
    let world = World::new();
    let (set_text, current_text) = world.mutable(String::from("Hello World!"));

    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::around(RTextView::new(current_text))
            .title("Cursive")
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}
