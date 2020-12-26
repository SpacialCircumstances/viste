use std::rc::Rc;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
pub use crate::values::*;

pub mod wires;
pub mod values;

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

pub struct RStream<'a, T>(Box<dyn Fn(T) -> () + 'a>);