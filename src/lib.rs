use std::rc::Rc;
use std::cell::{Ref, RefCell};
use std::ops::Deref;

pub mod combinators;
pub mod channels;

pub struct Pipe<'a, T>(Box<dyn Fn(&T) -> () + 'a>);

impl<'a, T> Pipe<'a, T> {
    pub fn new<F: Fn(&T) -> () + 'a>(fun: F) -> Self {
        Self(Box::new(fun))
    }

    pub fn run(&self, data: &T) {
        (self.0)(data)
    }

    pub fn cloneable(self) -> Rc<Self> {
        Rc::new(self)
    }
}

impl<'a, T: 'a> From<Rc<Pipe<'a, T>>>  for Pipe<'a, T> {
    fn from(l: Rc<Pipe<'a, T>>) -> Self {
        Pipe::new(move |t| l.run(t))
    }
}

impl<'a, T: 'a> From<Pipes<'a, T>> for Pipe<'a, T> {
    fn from(l: Pipes<'a, T>) -> Self {
        Pipe::new(move |t| l.distribute(t))
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

    pub fn distribute(&self, data: &T) {
        self.0.iter().for_each(|l| l.run(data));
    }
}

impl<'a, T> From<Pipe<'a, T>> for Pipes<'a, T> {
    fn from(p: Pipe<'a, T>) -> Self {
        Pipes::single(p)
    }
}

pub fn dead_end<'a, T>() -> Pipe<'a, T> {
    Pipe::new(|_| {})
}

pub struct RefWrapper<'a, T>(Ref<'a, T>);

impl<'a, T> Deref for RefWrapper<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub trait Rv<T>: Clone {
    fn data(&self) -> RefWrapper<T>;
}

pub trait RvExt<T>: Rv<T> {
    fn select<U, M: Fn(&T) -> &U + Clone>(&self, mapper: M) -> SelectRv<T, U, M>;
}

pub struct OwnedRv<T>(Rc<RefCell<T>>);

impl<T> Clone for OwnedRv<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Rv<T> for OwnedRv<T> {
    fn data(&self) -> RefWrapper<T> {
        RefWrapper(self.0.borrow())
    }
}

pub struct SelectRv<T, T2, M: Fn(&T) -> &T2 + Clone>(Rc<RefCell<T>>, M);

impl<T, T2, M: Fn(&T) -> &T2 + Clone> Clone for SelectRv<T, T2, M> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<T, T2, M: Fn(&T) -> &T2 + Clone> Rv<T2> for SelectRv<T, T2, M> {
    fn data(&self) -> RefWrapper<T2> {
        RefWrapper(Ref::map(self.0.borrow(), &self.1))
    }
}

pub fn store<'a, T: Copy + 'a>(default: T) -> (Pipe<'a, T>, OwnedRv<T>) {
    let store = Rc::new(RefCell::new(default));
    let c = store.clone();
    let pipe = Pipe::new(move |t| {
        c.replace(*t);
    });
    (pipe, OwnedRv(store))
}

pub fn store_clone<'a, T: Clone + 'a>(default: T) -> (Pipe<'a, T>, OwnedRv<T>) {
    let store = Rc::new(RefCell::new(default));
    let c = store.clone();
    let pipe = Pipe::new(move |t: &T| {
        c.replace(t.clone());
    });
    (pipe, OwnedRv(store))
}