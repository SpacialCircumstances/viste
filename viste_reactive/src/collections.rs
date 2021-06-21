use crate::*;

#[derive(Debug)]
pub enum SetChange<T: Data> {
    Added(T),
    Removed(T),
    Clear,
}

impl<T: Data> Data for SetChange<T> {
    fn changed(&self, other: &Self) -> bool {
        match (self, other) {
            (SetChange::Added(v1), SetChange::Added(v2)) => v1.changed(v2),
            (SetChange::Removed(v1), SetChange::Removed(v2)) => v1.changed(v2),
            (SetChange::Clear, SetChange::Clear) => false,
            _ => true,
        }
    }

    fn cheap_clone(&self) -> Self {
        match self {
            SetChange::Added(v) => SetChange::Added(v.cheap_clone()),
            SetChange::Removed(v) => SetChange::Removed(v.cheap_clone()),
            SetChange::Clear => SetChange::Clear,
        }
    }
}

pub struct CollectionSignal<'a, T: Data + 'a>(StreamSignal<'a, SetChange<T>>);

impl<'a, T: Data + 'a> Clone for CollectionSignal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T: Data + 'a> From<StreamSignal<'a, SetChange<T>>> for CollectionSignal<'a, T> {
    fn from(stream: StreamSignal<'a, SetChange<T>>) -> Self {
        CollectionSignal::new(stream)
    }
}

impl<'a, T: Data + 'a> CollectionSignal<'a, T> {
    pub fn new(stream: StreamSignal<'a, SetChange<T>>) -> Self {
        CollectionSignal(stream)
    }

    pub fn changes(&self) -> StreamSignal<'a, SetChange<T>> {
        self.0.clone()
    }

    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> CollectionSignal<'a, R> {
        self.0
            .map(move |c| match c {
                SetChange::Added(t) => SetChange::Added(mapper(t)),
                SetChange::Removed(t) => SetChange::Removed(mapper(t)),
                SetChange::Clear => SetChange::Clear,
            })
            .into()
    }
}

pub struct CollectionPortal<'a, T: Data + 'a> {
    signal: CollectionSignal<'a, T>,
    sender: Box<dyn Fn(SetChange<T>) + 'a>,
}

impl<'a, T: Data + 'a> CollectionPortal<'a, T> {
    pub fn new(world: &World) -> Self {
        let (sender, signal) = portal(world);
        CollectionPortal {
            sender: Box::new(sender),
            signal: CollectionSignal(signal),
        }
    }

    pub fn signal(&self) -> &CollectionSignal<'a, T> {
        &self.signal
    }

    pub fn add(&mut self, t: T) {
        (self.sender)(SetChange::Added(t))
    }

    pub fn remove(&mut self, t: T) {
        (self.sender)(SetChange::Removed(t))
    }

    pub fn clear(&mut self) {
        (self.sender)(SetChange::Clear)
    }
}
