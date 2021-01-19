use bracket_lib::prelude::{BTerm, Console, GameState};
use std::rc::Rc;
use viste_reactive::{OwnedRValue, RStream, RValue, RWire, RWires};

pub trait Component {
    fn draw(&self, ctx: &mut BTerm);
}

pub struct TextComponent<'a> {
    x: OwnedRValue<i32>,
    y: OwnedRValue<i32>,
    x_wire: RWires<'a, i32>,
    y_wire: RWires<'a, i32>,
    text: OwnedRValue<String>,
    text_stream: RStream<'a, String>,
}

impl<'a> TextComponent<'a> {
    pub fn new(x: i32, y: i32, text: String) -> Self {
        let (x_wire, x) = RWire::store(x);
        let (y_wire, y) = RWire::store(y);
        let (text_stream, text) = RStream::store(text);
        Self {
            x,
            y,
            text,
            x_wire: x_wire.into(),
            y_wire: y_wire.into(),
            text_stream,
        }
    }

    pub fn x_wire(&self) -> &RWires<i32> {
        &self.x_wire
    }

    pub fn y_wire(&self) -> &RWires<i32> {
        &self.y_wire
    }

    pub fn text_stream(&self) -> &RStream<String> {
        &self.text_stream
    }
}

impl<'a> Component for TextComponent<'a> {
    fn draw(&self, ctx: &mut BTerm) {
        ctx.print(*self.x.data(), *self.y.data(), &*self.text.data());
    }
}

pub struct ReactiveState {
    root: Rc<dyn Component>,
}

impl ReactiveState {
    pub fn new(root: Rc<dyn Component>) -> Self {
        Self { root }
    }
}

impl GameState for ReactiveState {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.root.draw(ctx);
    }
}
