use std::cell::Cell;
use std::rc::Rc;

struct Listeners<'a, T: Copy>(Vec<Box<dyn Fn(T) + 'a>>);

impl<'a, T: Copy> Listeners<'a, T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add<F: Fn(T) + 'a>(&mut self, listener: F) {
        self.0.push(Box::new(listener))
    }

    pub fn invoke(&self, value: T) {
        for l in &self.0 {
            (l)(value)
        }
    }
}

impl<'a, T: Copy> Default for Listeners<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SharedDirtyFlag<'a>(Rc<Cell<bool>>, Listeners<'a, ()>);

impl<'a> SharedDirtyFlag<'a> {
    pub fn new(value: bool) -> Self {
        Self(Rc::new(Cell::new(value)), Listeners::new())
    }

    pub fn add_listener<F: Fn(()) + 'a>(&mut self, listener: F) {
        self.1.add(listener);
    }

    pub fn is_dirty(&self) -> bool {
        self.0.get()
    }

    pub fn mark_dirty(&self) {
        if !self.is_dirty() {
            self.0.replace(true);
            self.1.invoke(())
        }
    }

    pub fn reset(&self) {
        self.0.replace(false);
    }
}

impl<'a> Default for SharedDirtyFlag<'a> {
    fn default() -> Self {
        Self::new(false)
    }
}

struct Node<'a, T> {
    dirty: bool,
    recompute: Box<dyn Fn() -> T + 'a>,
    current_value: T,
}

impl<'a, T> Node<'a, T> {
    pub fn new<F: Fn() -> T + 'a>(recompute: F) -> Self {
        let initial = recompute();
        Self {
            dirty: false,
            recompute: Box::new(recompute),
            current_value: initial,
        }
    }
}
