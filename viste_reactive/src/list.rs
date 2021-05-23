use crate::*;

#[derive(Debug)]
pub enum ListChange<T: Data> {
    Push(T),
    Remove(usize),
    Insert(T, usize),
    Clear,
    Swap(usize, usize),
}

impl<T: Data> Data for ListChange<T> {
    fn changed(&self, other: &Self) -> bool {
        match (self, other) {
            (ListChange::Push(v1), ListChange::Push(v2)) => v1.changed(v2),
            (ListChange::Insert(v1, i1), ListChange::Insert(v2, i2)) => i1 != i2 || v1.changed(v2),
            (ListChange::Clear, ListChange::Clear) => false,
            (ListChange::Swap(li1, li2), ListChange::Swap(ri1, ri2)) => li1 != ri1 || li2 != ri2,
            (ListChange::Remove(i1), ListChange::Remove(i2)) => i1 != i2,
            _ => true,
        }
    }

    fn cheap_clone(&self) -> Self {
        match self {
            ListChange::Push(v) => ListChange::Push(v.cheap_clone()),
            ListChange::Insert(v, i) => ListChange::Insert(v.cheap_clone(), *i),
            ListChange::Remove(i) => ListChange::Remove(*i),
            ListChange::Clear => ListChange::Clear,
            ListChange::Swap(i1, i2) => ListChange::Swap(*i1, *i2),
        }
    }
}

pub struct ListSignal<'a, T: Data + 'a>(StreamSignal<'a, ListChange<T>>);

impl<'a, T: Data + 'a> Clone for ListSignal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub fn lift<A: Data, B: Data, F: Fn(A) -> B>(func: F) -> impl Fn(ListChange<A>) -> ListChange<B> {
    move |t| match t {
        ListChange::Push(v) => ListChange::Push(func(v)),
        ListChange::Insert(v, i) => ListChange::Insert(func(v), i),
        ListChange::Clear => ListChange::Clear,
        ListChange::Remove(i) => ListChange::Remove(i),
        ListChange::Swap(i1, i2) => ListChange::Swap(i1, i2),
    }
}

impl<'a, T: Data + 'a> ListSignal<'a, T> {
    pub fn map<O: Data + 'a, M: Fn(T) -> O + 'a>(&self, mapper: M) -> ListSignal<'a, O> {
        ListSignal(self.0.map(lift(mapper)))
    }
}

pub struct RListSender<'a, T: Data + 'a> {
    signal: ListSignal<'a, T>,
    sender: Box<dyn Fn(ListChange<T>) + 'a>,
}

impl<'a, T: Data + 'a> RListSender<'a, T> {
    pub fn new(world: &World) -> Self {
        let (sender, signal) = portal(world);
        RListSender {
            sender: Box::new(sender),
            signal: ListSignal(signal),
        }
    }

    pub fn signal(&self) -> &ListSignal<'a, T> {
        &self.signal
    }

    pub fn push(&mut self, item: T) {
        (self.sender)(ListChange::Push(item))
    }

    pub fn clear(&mut self) {
        (self.sender)(ListChange::Clear)
    }

    pub fn remove(&mut self, idx: usize) {
        (self.sender)(ListChange::Remove(idx))
    }

    pub fn swap(&mut self, i1: usize, i2: usize) {
        (self.sender)(ListChange::Swap(i1, i2))
    }

    pub fn insert(&mut self, idx: usize, item: T) {
        (self.sender)(ListChange::Insert(item, idx))
    }
}

pub struct RList<'a, T: Data + 'a> {
    collector: Collector<'a, ListChange<T>>,
    store: Vec<T>,
}

impl<'a, T: Data> RList<'a, T> {
    pub fn new(signal: ListSignal<'a, T>) -> Self {
        Self {
            collector: signal.0.collect(),
            store: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        // TODO: Expose a drain method on collector
        let store = &mut self.store;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                ListChange::Push(item) => store.push(item),
                ListChange::Clear => store.clear(),
                ListChange::Insert(item, idx) => store.insert(idx, item),
                ListChange::Remove(idx) => {
                    store.remove(idx);
                }
                ListChange::Swap(i1, i2) => store.swap(i1, i2),
            });
        self.collector.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.store.iter()
    }

    pub fn store(&self) -> &Vec<T> {
        &self.store
    }
}
