use cursive::views::Dialog;
use std::rc::Rc;
use viste_cursive::RTextView;
use viste_reactive::events::{fold, Event};
use viste_reactive::signals::World;

#[derive(Copy, Clone, Debug)]
enum Msg {
    Incr,
    Decr,
}

fn main() {
    let world = World::new();
    let (dispatch, counter) = fold(
        &world,
        |msg, c| match msg {
            Msg::Incr => *c + 1,
            Msg::Decr => *c - 1,
        },
        0,
    );
    let current_text = counter.map(|c| Rc::new(format!("{}", c)));

    let mut siv = cursive::default();
    siv.set_user_data(dispatch);

    siv.add_layer(
        Dialog::around(RTextView::new(current_text))
            .title("Cursive")
            .button("+", |s| {
                s.with_user_data(|dispatch: &mut Event<Msg>| dispatch.push(Msg::Incr));
            })
            .button("-", |s| {
                s.with_user_data(|dispatch: &mut Event<Msg>| dispatch.push(Msg::Decr));
            })
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}
