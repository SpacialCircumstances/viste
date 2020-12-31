use cursive;
use cursive::views::{Dialog, TextView};

fn main() {
    let mut siv = cursive::default();

    let mut counter = 0;

    siv.add_layer(
        Dialog::around(TextView::new(format!("{}", counter)))
            .title("Counter")
            .button("Decr", |c| {})
            .button("Incr", |c| {}),
    );

    // Starts the event loop.
    siv.run();
}
