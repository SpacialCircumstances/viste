pub use crate::values::*;
use std::cell::{Ref, RefCell};
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc::{SendError, Sender};

pub mod lists;
pub mod streams;
pub mod values;
pub mod wires;

pub struct RWire<'a, T>(Box<dyn Fn(&T) + 'a>);

impl<'a, T: 'a> RWire<'a, T> {
    pub fn new<F: Fn(&T) + 'a>(fun: F) -> Self {
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

    pub fn from_borrowed(wires: &'a RWires<'a, T>) -> Self {
        Self::new(move |t| wires.distribute(t))
    }

    pub fn mapped<F: 'a, M: Fn(&F) -> T + 'a>(self, mapper: M) -> RWire<'a, F> {
        wires::combinators::map(mapper, self)
    }

    pub fn filtered<F: Fn(&T) -> bool + 'a>(self, filter: F) -> Self {
        wires::combinators::filter(filter, self)
    }

    pub fn filter_mapped<U: 'a, F: Fn(&U) -> Option<T> + 'a>(self, fm: F) -> RWire<'a, U> {
        wires::combinators::filter_map(fm, self)
    }

    pub fn reduced<U: 'a, R: Fn(&U, &mut T) + 'a>(self, reducer: R, initial: T) -> RWire<'a, U> {
        wires::combinators::reduce(reducer, initial, self)
    }

    pub fn cached(self) -> Self
    where
        T: Copy + Eq,
    {
        wires::combinators::cache(self)
    }

    pub fn cached_clone(self) -> Self
    where
        T: Clone + Eq,
    {
        wires::combinators::cache_clone(self)
    }

    pub fn cached_hash(self) -> Self
    where
        T: Hash,
    {
        wires::combinators::cache_hash(self)
    }

    pub fn store(default: T) -> (Self, OwnedRValue<T>)
    where
        T: Clone,
    {
        let store = Rc::new(RefCell::new(default));
        let c = store.clone();
        let pipe = RWire::new(move |t: &T| {
            c.replace(t.clone());
        });
        (pipe, OwnedRValue::new(store))
    }

    pub fn call_stream(stream: RStream<'a, T>) -> Self
    where
        T: Clone,
    {
        Self::new(move |t| stream.push(t.clone()))
    }

    pub fn send(sender: Sender<T>, result: RWires<'a, Result<(), SendError<T>>>) -> Self
    where
        T: Clone,
    {
        RWire::new(move |t: &T| {
            result.distribute(&sender.send(t.clone()));
        })
    }
}

impl<'a, T: 'a> From<&'a RWires<'a, T>> for RWire<'a, T> {
    fn from(wires: &'a RWires<'a, T>) -> Self {
        Self::from_borrowed(wires)
    }
}

impl<'a, T: 'a> From<Rc<RWire<'a, T>>> for RWire<'a, T> {
    fn from(l: Rc<RWire<'a, T>>) -> Self {
        RWire::new(move |t| l.run(t))
    }
}

impl<'a, T: 'a> From<Rc<RWires<'a, T>>> for RWire<'a, T> {
    fn from(l: Rc<RWires<'a, T>>) -> Self {
        RWire::new(move |t| l.distribute(t))
    }
}

impl<'a, T: 'a> From<RWires<'a, T>> for RWire<'a, T> {
    fn from(l: RWires<'a, T>) -> Self {
        RWire::new(move |t| l.distribute(t))
    }
}

pub struct RWires<'a, T>(Vec<RWire<'a, T>>);

impl<'a, T: 'a> RWires<'a, T> {
    pub fn new() -> Self {
        Self(Vec::with_capacity(1))
    }

    pub fn single(pipe: RWire<'a, T>) -> Self {
        RWires(vec![pipe])
    }

    pub fn distribute(&self, data: &T) {
        self.0.iter().for_each(|l| l.run(data));
    }

    pub fn add<I: Into<RWire<'a, T>>>(&mut self, wire: I) {
        self.0.push(wire.into())
    }

    pub fn mapped<F: 'a, M: Fn(&F) -> T + 'a>(self, mapper: M) -> RWire<'a, F> {
        wires::combinators::map(mapper, self)
    }

    pub fn filtered<F: Fn(&T) -> bool + 'a>(self, filter: F) -> Self {
        wires::combinators::filter(filter, self).into()
    }

    pub fn filter_mapped<U: 'a, F: Fn(&U) -> Option<T> + 'a>(self, fm: F) -> RWire<'a, U> {
        wires::combinators::filter_map(fm, self)
    }

    pub fn reduced<U: 'a, R: Fn(&U, &mut T) + 'a>(self, reducer: R, initial: T) -> RWire<'a, U> {
        wires::combinators::reduce(reducer, initial, self)
    }

    pub fn cached(self) -> Self
    where
        T: Copy + Eq,
    {
        wires::combinators::cache(self).into()
    }

    pub fn cached_clone(self) -> Self
    where
        T: Clone + Eq,
    {
        wires::combinators::cache_clone(self).into()
    }

    pub fn cached_hash(self) -> Self
    where
        T: Hash,
    {
        wires::combinators::cache_hash(self).into()
    }

    pub fn store(default: T) -> (Self, OwnedRValue<T>)
    where
        T: Clone,
    {
        let store = Rc::new(RefCell::new(default));
        let c = store.clone();
        let pipe = RWire::new(move |t: &T| {
            c.replace(t.clone());
        });
        (pipe.into(), OwnedRValue::new(store))
    }

    pub fn call_stream(stream: RStream<'a, T>) -> Self
    where
        T: Clone,
    {
        Self::single(RWire::new(move |t: &T| stream.push(t.clone())))
    }

    pub fn send(sender: Sender<T>, result: RWires<'a, Result<(), SendError<T>>>) -> Self
    where
        T: Clone,
    {
        RWire::new(move |t: &T| {
            result.distribute(&sender.send(t.clone()));
        })
        .into()
    }
}

impl<'a, T: 'a> Default for RWires<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: 'a> From<RWire<'a, T>> for RWires<'a, T> {
    fn from(p: RWire<'a, T>) -> Self {
        RWires::single(p)
    }
}

pub enum RefWrapper<'a, T> {
    Ref(Ref<'a, T>),
    Direct(&'a T),
}

impl<'a, T> RefWrapper<'a, T> {
    pub fn map<U, M: Fn(&T) -> &U>(self, mapper: M) -> RefWrapper<'a, U> {
        match self {
            RefWrapper::Ref(r) => RefWrapper::Ref(Ref::map(r, mapper)),
            RefWrapper::Direct(r) => RefWrapper::Direct(mapper(r)),
        }
    }
}

impl<'a, T> Deref for RefWrapper<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            RefWrapper::Ref(r) => r.deref(),
            RefWrapper::Direct(r) => r,
        }
    }
}

pub trait RValue<T>: Clone {
    fn data(&self) -> RefWrapper<T>;
}

pub struct RStream<'a, T: 'a>(Box<dyn Fn(T) + 'a>);

impl<'a, T: 'a> RStream<'a, T> {
    pub fn new<F: Fn(T) + 'a>(f: F) -> Self {
        Self(Box::new(f))
    }

    pub fn push(&self, value: T) {
        (self.0)(value)
    }

    pub fn dropping() -> Self {
        Self::new(|_x| {})
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

    pub fn send(sender: Sender<T>, result: RStream<'a, Result<(), SendError<T>>>) -> Self {
        RStream::new(move |t| {
            result.push(sender.send(t));
        })
    }

    pub fn mapped<F: 'a, M: Fn(F) -> T + 'a>(self, mapper: M) -> RStream<'a, F> {
        streams::combinators::map(mapper, self)
    }

    pub fn filtered<F: Fn(&T) -> bool + 'a>(self, filter: F) -> Self {
        streams::combinators::filter(filter, self)
    }

    pub fn filter_mapped<U: 'a, F: Fn(U) -> Option<T> + 'a>(self, fm: F) -> RStream<'a, U> {
        streams::combinators::filter_map(fm, self)
    }

    pub fn cached(self) -> Self
    where
        T: Copy + Eq,
    {
        streams::combinators::cache(self)
    }

    pub fn cached_clone(self) -> Self
    where
        T: Clone + Eq,
    {
        streams::combinators::cache_clone(self)
    }

    pub fn cached_hash(self) -> Self
    where
        T: Hash,
    {
        streams::combinators::cache_hash(self)
    }
}

impl<'a, T: 'a> From<RWires<'a, T>> for RStream<'a, T> {
    fn from(wires: RWires<'a, T>) -> Self {
        RStream::wires(wires)
    }
}

impl<'a, T: 'a> From<Rc<RStream<'a, T>>> for RStream<'a, T> {
    fn from(l: Rc<RStream<'a, T>>) -> Self {
        RStream::new(move |t| l.push(t))
    }
}
