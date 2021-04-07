use crate::Data;
use slab::Slab;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub trait Listener<T: Data> {
    fn call(&self, data: &T);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ListenerToken(usize);

pub trait Producer<T: Data> {
    fn add_listener<I: Into<Box<dyn Listener<T>>>>(&self, listener: I) -> ListenerToken;
    fn remove_listener(&self, listener: ListenerToken);
}

pub struct Listeners<T: Data>(Slab<Box<dyn Listener<T>>>);

impl<T: Data> Listeners<T> {
    pub fn new() -> Self {
        Self(Slab::new())
    }

    pub fn call_all(&self, data: &T) {
        self.0.iter().for_each(|(_, l)| l.call(data));
    }

    pub fn add_listener(&mut self, listener: Box<dyn Listener<T>>) -> ListenerToken {
        ListenerToken(self.0.insert(listener))
    }

    pub fn remove_listener(&mut self, listener: ListenerToken) {
        self.0.remove(listener.0);
    }
}

pub struct Sender<T: Data>(Rc<RefCell<Listeners<T>>>);

impl<T: Data> Sender<T> {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(Listeners::new())))
    }
}

impl<T: Data> Producer<T> for Sender<T> {
    fn add_listener<I: Into<Box<dyn Listener<T>>>>(&self, listener: I) -> ListenerToken {
        self.0.borrow_mut().add_listener(listener.into())
    }

    fn remove_listener(&self, listener: ListenerToken) {
        self.0.borrow_mut().remove_listener(listener)
    }
}

impl<T: Data> Listener<T> for Sender<T> {
    fn call(&self, data: &T) {
        self.0.borrow().call_all(data)
    }
}
