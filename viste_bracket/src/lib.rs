use bracket_lib::prelude::{BTerm, Console, GameState, VirtualKeyCode};
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

pub enum Event {
    KeyPressed(VirtualKeyCode),
    KeyReleased(VirtualKeyCode),
    LeftMouseDown(i32, i32),
    LeftMouseUp(i32, i32),
    MouseMoved { from: (i32, i32), to: (i32, i32) },
}

pub struct ReactiveState {
    root: Rc<dyn Component>,
    event_stream: RStream<'static, Event>,
    last_key: Option<VirtualKeyCode>,
    last_mouse: (i32, i32),
    last_left_click: bool,
}

impl ReactiveState {
    pub fn new(root: Rc<dyn Component>, event_stream: RStream<'static, Event>) -> Self {
        Self {
            root,
            event_stream,
            last_key: None,
            last_mouse: (0, 0),
            last_left_click: false,
        }
    }
}

impl GameState for ReactiveState {
    fn tick(&mut self, ctx: &mut BTerm) {
        match (self.last_key, ctx.key) {
            (None, Some(k)) => self.event_stream.push(Event::KeyPressed(k)),
            (Some(k), None) => self.event_stream.push(Event::KeyReleased(k)),
            _ => (),
        };
        self.last_key = ctx.key;

        if self.last_mouse != ctx.mouse_pos {
            self.event_stream.push(Event::MouseMoved {
                from: self.last_mouse,
                to: ctx.mouse_pos,
            });
            self.last_mouse = ctx.mouse_pos;
        }

        match (self.last_left_click, ctx.left_click) {
            (true, false) => self
                .event_stream
                .push(Event::LeftMouseUp(ctx.mouse_pos.0, ctx.mouse_pos.1)),
            (false, true) => self
                .event_stream
                .push(Event::LeftMouseDown(ctx.mouse_pos.0, ctx.mouse_pos.1)),
            _ => (),
        };
        self.last_left_click = ctx.left_click;

        self.root.draw(ctx);
    }
}
