use std::rc::Rc;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
pub use crate::values::*;
use std::sync::mpsc::{Sender, SendError};

pub mod wires;
pub mod values;
pub mod streams;

pub struct RWire<'a, T>(Box<dyn Fn(&T) -> () + 'a>);

impl<'a, T> RWire<'a, T> {
    pub fn new<F: Fn(&T) -> () + 'a>(fun: F) -> Self {
        Self(Box::new(fun))
    }

    pub fn run(&self, data: &T) {
        (self.0)(data)
    }

    pub fn cloneable(self) -> Rc<Self> {
        Rc::new(self)
    }

    pub fn dead() -> Self {
        Self::new(|_| {})
    }
}

impl<'a, T: Copy + 'a> RWire<'a, T> {
    pub fn store(default: T) -> (Self, OwnedRValue<T>) {
        let store = Rc::new(RefCell::new(default));
        let c = store.clone();
        let pipe = RWire::new(move |t| {
            c.replace(*t);
        });
        (pipe, OwnedRValue::new(store))
    }

    pub fn call_stream(stream: RStream<'a, T>) -> Self {
        Self::new(move |t| stream.push(*t))
    }

    pub fn send(sender: Sender<T>, result: RWires<'a, Result<(), SendError<T>>>) -> RWire<'a, T> {
        RWire::new(move |t| {
            result.distribute(&sender.send(*t));
        })
    }
}

impl<'a, T: Clone + 'a> RWire<'a, T> {
    pub fn store_clone(default: T) -> (RWire<'a, T>, OwnedRValue<T>) {
        let store = Rc::new(RefCell::new(default));
        let c = store.clone();
        let pipe = RWire::new(move |t: &T| {
            c.replace(t.clone());
        });
        (pipe, OwnedRValue::new(store))
    }

    pub fn call_stream_clone(stream: RStream<'a, T>) -> Self {
        Self::new(move |t| stream.push(t.clone()))
    }

    pub fn send_clone(sender: Sender<T>, result: RWires<'a, Result<(), SendError<T>>>) -> RWire<'a, T> {
        RWire::new(move |t: &T| {
            result.distribute(&sender.send(t.clone()));
        })
    }
}

impl<'a, T: 'a> From<Rc<RWire<'a, T>>>  for RWire<'a, T> {
    fn from(l: Rc<RWire<'a, T>>) -> Self {
        RWire::new(move |t| l.run(t))
    }
}

impl<'a, T: 'a> From<RWires<'a, T>> for RWire<'a, T> {
    fn from(l: RWires<'a, T>) -> Self {
        RWire::new(move |t| l.distribute(t))
    }
}

pub struct RWires<'a, T>(Vec<RWire<'a, T>>);

impl<'a, T> RWires<'a, T> {
    pub fn new() -> Self {
        Self(Vec::with_capacity(1))
    }

    pub fn single(pipe: RWire<'a, T>) -> Self {
        RWires(vec![ pipe ])
    }

    pub fn distribute(&self, data: &T) {
        self.0.iter().for_each(|l| l.run(data));
    }
}

impl<'a, T> From<RWire<'a, T>> for RWires<'a, T> {
    fn from(p: RWire<'a, T>) -> Self {
        RWires::single(p)
    }
}

pub struct RefWrapper<'a, T>(Ref<'a, T>);

impl<'a, T> Deref for RefWrapper<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub trait RValue<T>: Clone {
    fn data(&self) -> RefWrapper<T>;
}

pub struct RStream<'a, T: 'a>(Box<dyn Fn(T) -> () + 'a>);

impl<'a, T: 'a> RStream<'a, T> {
    pub fn new<F: Fn(T) -> () + 'a>(f: F) -> Self {
        Self(Box::new(f))
    }

    pub fn push(&self, value: T) {
        (self.0)(value)
    }

    pub fn dropping() -> Self {
        Self::new(|x| drop(x))
    }

    pub fn wires(wires: RWires<'a, T>) -> Self {
        Self::new(move |t| wires.distribute(&t))
    }

    pub fn store(default: T) -> (Self, OwnedRValue<T>) {
        let store = Rc::new(RefCell::new(default));
        let c = store.clone();
        let stream = RStream::new(move |t| {
            c.replace(t);
        });
        (stream, OwnedRValue::new(store))
    }

    pub fn cloneable(self) -> Rc<Self> {
        Rc::new(self)
    }
}

impl<'a, T: 'a> From<Rc<RStream<'a, T>>>  for RStream<'a, T> {
    fn from(l: Rc<RStream<'a, T>>) -> Self {
        RStream::new(move |t| l.push(t))
    }
}