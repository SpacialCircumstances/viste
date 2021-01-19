use bracket_lib::prelude::*;
use viste_bracket::{ReactiveState, TextComponent};

fn main() {
    let context = BTermBuilder::simple80x50()
        .with_title("Hello World")
        .build();

    let text = TextComponent::new(0, 0, "Hello World!".into());

    let state = ReactiveState::new(Box::new(text));
    main_loop(context, state);
}
