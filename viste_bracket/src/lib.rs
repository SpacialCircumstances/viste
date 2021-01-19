use bracket_lib::prelude::{BTerm, GameState};

pub trait Component {
    fn draw(&mut self, ctx: &mut BTerm);
}

type ComponentBox = Box<dyn Component>;

pub struct ReactiveState {
    root: ComponentBox,
}

impl GameState for ReactiveState {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.root.draw(ctx);
    }
}
