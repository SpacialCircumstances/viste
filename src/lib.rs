use std::rc::Rc;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::marker::PhantomData;

pub mod combinators;
pub mod channels;

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

pub fn dead_end<'a, T>() -> RWire<'a, T> {
    RWire::new(|_| {})
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

pub trait RValueExt<T>: RValue<T> {
    fn select<U, M: Fn(&T) -> &U + Clone>(&self, mapper: M) -> SelectRValue<T, U, M, Self> {
        SelectRValue(self.clone(), mapper, PhantomData::default())
    }
}

impl<T, R: RValue<T>> RValueExt<T> for R {

}

pub struct OwnedRValue<T>(Rc<RefCell<T>>);

impl<T> Clone for OwnedRValue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> RValue<T> for OwnedRValue<T> {
    fn data(&self) -> RefWrapper<T> {
        RefWrapper(self.0.borrow())
    }
}

pub struct SelectRValue<T, T2, M: Fn(&T) -> &T2 + Clone, R: RValue<T>>(R, M, PhantomData<T>);

impl<T, T2, M: Fn(&T) -> &T2 + Clone, R: RValue<T>> Clone for SelectRValue<T, T2, M, R> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData::default())
    }
}

impl<T, T2, M: Fn(&T) -> &T2 + Clone, R: RValue<T>> RValue<T2> for SelectRValue<T, T2, M, R> {
    fn data(&self) -> RefWrapper<T2> {
        RefWrapper(Ref::map(self.0.data().0, &self.1))
    }
}

pub fn store<'a, T: Copy + 'a>(default: T) -> (RWire<'a, T>, OwnedRValue<T>) {
    let store = Rc::new(RefCell::new(default));
    let c = store.clone();
    let pipe = RWire::new(move |t| {
        c.replace(*t);
    });
    (pipe, OwnedRValue(store))
}

pub fn store_clone<'a, T: Clone + 'a>(default: T) -> (RWire<'a, T>, OwnedRValue<T>) {
    let store = Rc::new(RefCell::new(default));
    let c = store.clone();
    let pipe = RWire::new(move |t: &T| {
        c.replace(t.clone());
    });
    (pipe, OwnedRValue(store))
}

pub struct RStream<'a, T>(Box<dyn Fn(T) -> () + 'a>);