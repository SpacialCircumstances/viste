use crate::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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

    pub fn data(&self) -> &BTreeSet<T> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

pub struct HashMapView<'a, T: Data + 'a, K: Hash + Eq + 'a, V: 'a> {
    collector: Collector<'a, SetChange<T>>,
    key_func: Box<dyn Fn(&T) -> K + 'a>,
    value_func: Box<dyn Fn(T) -> V + 'a>,
    data: HashMap<K, V>,
}

impl<'a, T: Data + 'a, K: Hash + Eq + 'a, V: 'a> HashMapView<'a, T, K, V> {
    pub fn new<KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        signal: CollectionSignal<'a, T>,
        key_func: KF,
        value_func: VF,
    ) -> Self {
        Self {
            collector: signal.0.collect(),
            data: HashMap::new(),
            key_func: Box::new(key_func),
            value_func: Box::new(value_func),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;
        let kf = &self.key_func;
        let vf = &self.value_func;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    store.insert((kf)(&t), (vf)(t));
                }
                SetChange::Removed(t) => {
                    store.remove(&(kf)(&t));
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    pub fn data(&self) -> &HashMap<K, V> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.data.iter()
    }
}

pub struct BTreeMapView<'a, T: Data + 'a, K: Ord + Eq + 'a, V: 'a> {
    collector: Collector<'a, SetChange<T>>,
    key_func: Box<dyn Fn(&T) -> K + 'a>,
    value_func: Box<dyn Fn(T) -> V + 'a>,
    data: BTreeMap<K, V>,
}

impl<'a, T: Data + 'a, K: Data + Ord + Eq + 'a, V: Data + 'a> BTreeMapView<'a, T, K, V> {
    pub fn new<KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        signal: CollectionSignal<'a, T>,
        key_func: KF,
        value_func: VF,
    ) -> Self {
        Self {
            collector: signal.0.collect(),
            data: BTreeMap::new(),
            key_func: Box::new(key_func),
            value_func: Box::new(value_func),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;
        let kf = &self.key_func;
        let vf = &self.value_func;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    store.insert((kf)(&t), (vf)(t));
                }
                SetChange::Removed(t) => {
                    store.remove(&(kf)(&t));
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    pub fn data(&self) -> &BTreeMap<K, V> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.data.iter()
    }
}

pub struct VecIndexView<'a, T: Data + 'a> {
    collector: Collector<'a, SetChange<T>>,
    index_func: Box<dyn Fn(&T) -> usize + 'a>,
    data: Vec<Option<T>>,
}

impl<'a, T: Data + 'a> VecIndexView<'a, T> {
    pub fn new<IF: Fn(&T) -> usize + 'a>(signal: CollectionSignal<'a, T>, index_func: IF) -> Self {
        Self {
            collector: signal.0.collect(),
            data: Vec::new(),
            index_func: Box::new(index_func),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;
        let idxf = &self.index_func;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    let idx: usize = idxf(&t);
                    if store.len() <= idx {
                        store.resize_with(idx + 1, || None);
                    }
                    store[idx] = Some(t);
                }
                SetChange::Removed(t) => {
                    store[idxf(&t)] = None;
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    pub fn data(&self) -> &Vec<Option<T>> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &Option<T>> {
        self.data.iter()
    }
}

pub struct OrderedVecView<'a, T: Data + 'a, K: Copy + Eq + Ord + 'a> {
    data: Vec<(K, T)>,
    key_func: Box<dyn Fn(&T) -> K + 'a>,
    collector: Collector<'a, SetChange<T>>,
}

impl<'a, T: Data + 'a, K: Copy + Eq + Ord + 'a> OrderedVecView<'a, T, K> {
    pub fn new<KF: Fn(&T) -> K + 'a>(signal: CollectionSignal<'a, T>, key_func: KF) -> Self {
        OrderedVecView {
            data: Vec::new(),
            key_func: Box::new(key_func),
            collector: signal.0.collect(),
        }
    }

    pub fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;
        let keyf = &self.key_func;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    let key = keyf(&t);
                    match store.binary_search_by_key(&key, |(k, _)| *k) {
                        Ok(existing_idx) => store[existing_idx] = (key, t),
                        Err(new_idx) => store.insert(new_idx, (key, t)),
                    }
                }
                SetChange::Removed(t) => {
                    let key = keyf(&t);
                    match store.binary_search_by_key(&key, |(k, _)| *k) {
                        Ok(existing_idx) => {
                            store.remove(existing_idx);
                        }
                        Err(_) => (), //Should we panic?
                    }
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    pub fn data(&self) -> &Vec<(K, T)> {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &(K, T)> {
        self.data.iter()
    }
}
