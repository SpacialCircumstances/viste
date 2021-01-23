use bracket_lib::prelude::*;
use std::rc::Rc;
use viste_bracket::{ReactiveState, TextComponent};
use viste_reactive::RStream;

fn main() {
    let context = BTermBuilder::simple80x50()
        .with_title("Hello World")
        .build();

    let text = Rc::new(TextComponent::new(0, 0, "Hello World!".into()));

    let state = ReactiveState::new(text.clone(), RStream::dropping());
    main_loop(context, state);
}
