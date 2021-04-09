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
    fn add_listener<Il: Into<Box<dyn Listener<T>>>>(&self, listener: Il) -> ListenerToken;
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

    pub fn send(&self, data: &T) {
        self.0.borrow().call_all(data)
    }
}

impl<T: Data> Producer<T> for Sender<T> {
    fn add_listener<Il: Into<Box<dyn Listener<T>>>>(&self, listener: Il) -> ListenerToken {
        self.0.borrow_mut().add_listener(listener.into())
    }

    fn remove_listener(&self, listener: ListenerToken) {
        self.0.borrow_mut().remove_listener(listener)
    }
}

struct EventCore<I: Data, O: Data> {
    compute: Box<dyn Fn(&I, &mut Listeners<O>)>,
    listeners: RefCell<Listeners<O>>,
}

pub struct EventStream<I: Data, O: Data>(Rc<EventCore<I, O>>);

pub struct EventListener<I: Data, O: Data>(Weak<EventCore<I, O>>);

impl<I: Data, O: Data> Listener<I> for EventListener<I, O> {
    fn call(&self, data: &I) {
        let ev = self.0.upgrade().expect("Failed to get event core");
        let mut listeners = ev.listeners.borrow_mut();
        (ev.compute)(data, &mut listeners)
    }
}

impl<I: Data, O: Data> EventStream<I, O> {
    pub fn listener(&self) -> EventListener<I, O> {
        EventListener(Rc::downgrade(&self.0))
    }
}

impl<I: Data, O: Data> Producer<O> for EventStream<I, O> {
    fn add_listener<Il: Into<Box<dyn Listener<O>>>>(&self, listener: Il) -> ListenerToken {
        self.0.listeners.borrow_mut().add_listener(listener.into())
    }

    fn remove_listener(&self, listener: ListenerToken) {
        self.0.listeners.borrow_mut().remove_listener(listener)
    }
}
