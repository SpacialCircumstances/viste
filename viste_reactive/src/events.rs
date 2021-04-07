use crate::Data;
use std::rc::{Rc, Weak};

pub struct EventCore<T>(dyn Fn(&T) -> ());

pub trait Listener<T: Data> {
    fn call(&self, data: &T);
}

pub struct EventListener<T: Data>(Weak<EventCore<T>>);

impl<T: Data> EventListener<T> {
    fn upgrade(&self) -> Event<T> {
        Event(self.0.upgrade().expect("Failed to upgrade listener"))
    }
}

impl<T: Data> Listener<T> for EventListener<T> {
    fn call(&self, data: &T) {
        self.upgrade().send(data);
    }
}

pub struct ListenerToken(usize);

pub struct Event<T: Data>(Rc<EventCore<T>>);

impl<T: Data> Event<T> {
    fn to_listener(&self) -> EventListener<T> {
        EventListener(Rc::downgrade(&self.0))
    }
}

impl<T: Data> Event<T> {
    pub fn send(&self, value: &T) {
        (self.0 .0)(value)
    }

    pub fn add_listener<L: Listener<T>>(&self, listener: L) -> ListenerToken {
        unimplemented!()
    }

    pub fn remove_listener(&self, listener: ListenerToken) {
        unimplemented!()
    }
}
