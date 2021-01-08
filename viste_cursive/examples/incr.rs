use cursive;
use cursive::views::Dialog;
use viste_cursive::views::text_view::RTextView;
use viste_reactive::wires::combinators::reduce;
use viste_reactive::RWire;

enum Msg {
    Incr,
    Decr,
}

fn main() {
    let mut siv = cursive::default();

    let mut text = RTextView::new();
    let set_text = text.set();
    let counter = set_text.mapped(|i| format!("{}", i));
    let dispatch = reduce(
        |msg, state| match msg {
            Msg::Incr => *state += 1,
            Msg::Decr => *state -= 1,
        },
        0,
        counter,
    )
    .cloneable();

    let d1 = dispatch.clone();
    let d2 = dispatch.clone();
    siv.add_layer(
        Dialog::around(text)
            .title("Counter")
            .button("Decr", move |_c| d1.run(&Msg::Decr))
            .button("Incr", move |_c| d2.run(&Msg::Incr)),
    );

    // Starts the event loop.
    siv.run();
}
