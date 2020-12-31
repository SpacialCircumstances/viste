use cursive;
use cursive::views::{Dialog, TextContent, TextView};
use viste_reactive::wires::combinators::reduce;
use viste_reactive::RWire;

enum Msg {
    Incr,
    Decr,
}

fn main() {
    let mut siv = cursive::default();

    let counter_content = TextContent::new("");
    let co = counter_content.clone();
    let counter_str = RWire::new(move |s| co.set_content(s));
    let counter = counter_str.mapped(|i| format!("{}", i));
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
        Dialog::around(TextView::new_with_content(counter_content.clone()))
            .title("Counter")
            .button("Decr", move |_c| d1.run(&Msg::Decr))
            .button("Incr", move |_c| d2.run(&Msg::Incr)),
    );

    // Starts the event loop.
    siv.run();
}
