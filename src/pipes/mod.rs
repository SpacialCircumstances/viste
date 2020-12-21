use std::rc::Rc;
use std::cell::{RefCell, Cell};

pub mod combinators;
pub mod channels;

pub struct Pipe<'a, T>(Box<dyn Fn(&T) -> () + 'a>);

impl<'a, T> Pipe<'a, T> {
    pub fn new<F: Fn(&T) -> () + 'a>(fun: F) -> Self {
        Self(Box::new(fun))
    }

    pub fn invoke(&self, data: &T) {
        (self.0)(data)
    }

    pub fn cloneable(self) -> Rc<Self> {
        Rc::new(self)
    }
}

impl<'a, T: 'a> From<Rc<Pipe<'a, T>>>  for Pipe<'a, T> {
    fn from(l: Rc<Pipe<'a, T>>) -> Self {
        Pipe::new(move |t| l.invoke(t))
    }
}

impl<'a, T: 'a> From<Pipes<'a, T>> for Pipe<'a, T> {
    fn from(l: Pipes<'a, T>) -> Self {
        Pipe::new(move |t| l.notify_all(t))
    }
}

pub struct Pipes<'a, T>(Vec<Pipe<'a, T>>);

impl<'a, T> Pipes<'a, T> {
    pub fn new() -> Self {
        Self(Vec::with_capacity(1))
    }

    pub fn single(pipe: Pipe<'a, T>) -> Self {
        Pipes(vec![ pipe ])
    }

    pub fn notify_all(&self, data: &T) {
        self.0.iter().for_each(|l| l.invoke(data));
    }
}

impl<'a, T> From<Pipe<'a, T>> for Pipes<'a, T> {
    fn from(p: Pipe<'a, T>) -> Self {
        Pipes::single(p)
    }
}

pub fn store<'a, T: Copy + 'a>(default: T) -> (Pipe<'a, T>, Rc<Cell<T>>) {
    let store = Rc::new(Cell::new(default));
    let c = store.clone();
    let pipe = Pipe::new(move |t| {
        c.set(*t)
    });
    (pipe, store)
}

pub fn store_clone<'a, T: Clone + 'a>(default: T) -> (Pipe<'a, T>, Rc<RefCell<T>>) {
    let store = Rc::new(RefCell::new(default));
    let c = store.clone();
    let pipe = Pipe::new(move |t: &T| {
        c.replace(t.clone());
    });
    (pipe, store)
}