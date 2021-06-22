use crate::*;
use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

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

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F) -> CollectionSignal<'a, T> {
        self.0
            .filter(move |c| match c {
                SetChange::Added(t) => filter(t),
                SetChange::Removed(t) => filter(t),
                SetChange::Clear => true,
            })
            .into()
    }

    pub fn filter_map<O: Data + 'a, F: Fn(T) -> Option<O> + 'a>(
        &self,
        f: F,
    ) -> CollectionSignal<'a, O> {
        self.0
            .filter_map(move |c| match c {
                SetChange::Added(t) => f(t).map(|o| SetChange::Added(o)),
                SetChange::Removed(t) => f(t).map(|o| SetChange::Removed(o)),
                SetChange::Clear => Some(SetChange::Clear),
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

pub struct HashSetView<'a, T: Data + Hash + Eq + 'a> {
    collector: Collector<'a, SetChange<T>>,
    data: HashSet<T>,
}

impl<'a, T: Data + Hash + Eq + 'a> HashSetView<'a, T> {
    pub fn new(signal: CollectionSignal<'a, T>) -> Self {
        Self {
            collector: signal.0.collect(),
            data: HashSet::new(),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    store.insert(t);
                }
                SetChange::Removed(t) => {
                    store.remove(&t);
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    pub fn data(&self) -> &HashSet<T> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

pub struct BTreeView<'a, T: Data + Eq + Ord + 'a> {
    collector: Collector<'a, SetChange<T>>,
    data: BTreeSet<T>,
}

impl<'a, T: Data + Eq + Ord + 'a> BTreeView<'a, T> {
    pub fn new(signal: CollectionSignal<'a, T>) -> Self {
        Self {
            collector: signal.0.collect(),
            data: BTreeSet::new(),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    store.insert(t);
                }
                SetChange::Removed(t) => {
                    store.remove(&t);
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    pub fn data(&self) -> &BtreeSet<T> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}
