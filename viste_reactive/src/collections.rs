use crate::streams::filter_mapper::FilterMapper;
use crate::*;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq)]
pub enum SetChange<T: Data> {
    Added(T),
    Removed(T),
    Clear,
}

pub trait View<'a, T: Data + 'a> {
    type Item;
    fn update(&mut self);
    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &'b Self::Item> + 'b>;
    fn iter<'b>(&'b mut self) -> Box<dyn Iterator<Item = &'b Self::Item> + 'b> {
        self.update();
        self.iter_unchanged()
    }
}

pub trait DirectView<'a, T: Data + 'a>: View<'a, T, Item = T> {
    fn new(collector: Collector<'a, SetChange<T>>) -> Self;
}

struct StateItems<Item: Data>(HashMap<ReaderToken, VecDeque<Item>>);

impl<Item: Data> StateItems<Item> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, reader: ReaderToken, items: VecDeque<Item>) {
        if !items.is_empty() {
            self.0.insert(reader, items);
        }
    }

    pub fn remove(&mut self, reader: ReaderToken) {
        self.0.remove(&reader);
    }

    pub fn get_next(&mut self, reader: ReaderToken) -> Option<Item> {
        match self.0.entry(reader) {
            Entry::Vacant(_) => None,
            Entry::Occupied(mut entry) => {
                let deq = entry.get_mut();
                let next = deq.pop_front();
                if deq.is_empty() {
                    entry.remove();
                }
                next
            }
        }
    }
}

pub struct CollectionComputationCore<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> {
    stream_signal: Signal<'a, Option<SetChange<T>>>,
    view: D,
    state_items: StateItems<SetChange<T>>,
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> CollectionComputationCore<'a, T, D> {
    pub fn new(signal: Signal<'a, Option<SetChange<T>>>) -> Self {
        Self {
            stream_signal: signal.clone(),
            view: D::new(signal.collect()),
            state_items: StateItems::new(),
        }
    }

    pub fn iter_view_items(&mut self) -> impl Iterator<Item = &T> {
        self.view.iter()
    }
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> ComputationCore
    for CollectionComputationCore<'a, T, D>
{
    type ComputationResult = Option<SetChange<T>>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        self.state_items
            .get_next(reader)
            .or_else(|| self.stream_signal.compute(reader))
    }

    fn create_reader(&mut self) -> ReaderToken {
        let r = self.stream_signal.create_reader();
        let items = self
            .iter_view_items()
            .map(|t| SetChange::Added(t.cheap_clone()))
            .collect();
        self.state_items.insert(r, items);
        r
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.state_items.remove(reader);
        self.stream_signal.destroy_reader(reader)
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.stream_signal.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.stream_signal.remove_dependency(child)
    }

    fn is_dirty(&self) -> bool {
        self.stream_signal.is_dirty()
    }

    fn world(&self) -> World {
        self.stream_signal.world()
    }

    fn node(&self) -> NodeIndex {
        self.stream_signal.node()
    }
}

pub struct CollectionSignal<'a, T: Data + 'a>(Signal<'a, Option<SetChange<T>>>);

impl<'a, T: Data + 'a> CollectionSignal<'a, T> {
    pub fn new<D: DirectView<'a, T> + 'a>(signal: StreamSignal<'a, SetChange<T>>) -> Self {
        let core: CollectionComputationCore<T, D> = CollectionComputationCore::new(signal.0);
        Self(Signal::create(core))
    }

    pub fn create<C: ComputationCore<ComputationResult = Option<SetChange<T>>> + 'a>(
        core: C,
    ) -> Self {
        CollectionSignal(Signal::create(
            CollectionComputationCore::<T, VecView<T>>::new(Signal::create(core)),
        ))
    }

    pub fn signal(&self) -> &Signal<'a, Option<SetChange<T>>> {
        &self.0
    }

    pub fn to_signal(self) -> Signal<'a, Option<SetChange<T>>> {
        self.0
    }

    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> CollectionSignal<'a, R> {
        CollectionSignal(Signal::create(
            CollectionComputationCore::<R, VecView<R>>::new(Signal::create(
                streams::mapper::Mapper::new(
                    self.signal().world(),
                    self.0.clone(),
                    move |c| match c {
                        SetChange::Added(t) => SetChange::Added(mapper(t)),
                        SetChange::Removed(t) => SetChange::Removed(mapper(t)),
                        SetChange::Clear => SetChange::Clear,
                    },
                ),
            )),
        ))
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F) -> CollectionSignal<'a, T> {
        CollectionSignal::create(streams::filter::Filter::new(
            self.signal().world(),
            self.0.clone(),
            move |c| match c {
                SetChange::Added(t) => filter(t),
                SetChange::Removed(t) => filter(t),
                SetChange::Clear => true,
            },
        ))
    }

    pub fn filter_map<O: Data + 'a, F: Fn(T) -> Option<O> + 'a, D2: DirectView<'a, O>>(
        &self,
        f: F,
    ) -> CollectionSignal<'a, O> {
        CollectionSignal::create(FilterMapper::new(
            self.signal().world(),
            self.0.clone(),
            move |c| match c {
                SetChange::Added(t) => f(t).map(SetChange::Added),
                SetChange::Removed(t) => f(t).map(SetChange::Removed),
                SetChange::Clear => Some(SetChange::Clear),
            },
        ))
    }

    pub fn collect(&self) -> Collector<'a, SetChange<T>> {
        self.signal().collect()
    }

    pub fn view_set_hash(&self) -> HashSetView<'a, T>
    where
        T: Hash + Eq,
    {
        HashSetView::new(self.signal().collect())
    }

    pub fn view_set_btree(&self) -> BTreeSetView<'a, T>
    where
        T: Ord + Eq,
    {
        BTreeSetView::new(self.collect())
    }

    pub fn view_map_hash<K: Hash + Eq + 'a, V: 'a, KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        &self,
        key_func: KF,
        value_func: VF,
    ) -> HashMapView<'a, T, K, V> {
        HashMapView::new(self.collect(), key_func, value_func)
    }

    pub fn view_map_btree<K: Ord + Eq + 'a, V: 'a, KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        &self,
        key_func: KF,
        value_func: VF,
    ) -> BTreeMapView<'a, T, K, V> {
        BTreeMapView::new(self.collect(), key_func, value_func)
    }

    pub fn view_vec_indexed<R: 'a, IF: Fn(&T) -> usize + 'a, VF: Fn(T) -> R + 'a>(
        &self,
        index_func: IF,
        value_func: VF,
    ) -> VecIndexView<'a, T, R> {
        VecIndexView::new(self.collect(), index_func, value_func)
    }

    pub fn view_vec_sorted<K: Copy + Ord + Eq + Data, KF: Fn(&T) -> K + 'a>(
        &self,
        key_func: KF,
    ) -> OrderedVecView<'a, T, K> {
        OrderedVecView::new(self.collect(), key_func)
    }

    pub fn view_vec(&self) -> VecView<'a, T>
    where
        T: PartialEq,
    {
        VecView::new(self.collect())
    }
}

pub struct CollectionPortal<'a, T: Data + 'a> {
    signal: CollectionSignal<'a, T>,
    sender: Box<dyn Fn(SetChange<T>) + 'a>,
}

impl<'a, T: Data + 'a> CollectionPortal<'a, T> {
    pub fn new<D: DirectView<'a, T> + 'a>(world: &World) -> Self {
        let (sender, signal) = portal(world);
        CollectionPortal {
            sender: Box::new(sender),
            signal: CollectionSignal::new::<D>(signal),
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

impl<'a, T: Data + Hash + Eq + 'a> View<'a, T> for HashSetView<'a, T> {
    type Item = T;

    fn update(&mut self) {
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

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.iter())
    }
}

impl<'a, T: Data + Hash + Eq + 'a> DirectView<'a, T> for HashSetView<'a, T> {
    fn new(collector: Collector<'a, SetChange<T>>) -> Self {
        Self {
            data: HashSet::new(),
            collector,
        }
    }
}

impl<'a, T: Data + Hash + Eq + 'a> HashSetView<'a, T> {
    pub fn data_unchanged(&self) -> &HashSet<T> {
        &self.data
    }

    pub fn data(&mut self) -> &HashSet<T> {
        self.update();
        self.data_unchanged()
    }
}

pub struct BTreeSetView<'a, T: Data + Eq + Ord + 'a> {
    collector: Collector<'a, SetChange<T>>,
    data: BTreeSet<T>,
}

impl<'a, T: Data + Eq + Ord + 'a> BTreeSetView<'a, T> {
    pub fn data_unchanged(&self) -> &BTreeSet<T> {
        &self.data
    }

    pub fn data(&mut self) -> &BTreeSet<T> {
        self.update();
        self.data_unchanged()
    }
}

impl<'a, T: Data + Eq + Ord + 'a> View<'a, T> for BTreeSetView<'a, T> {
    type Item = T;

    fn update(&mut self) {
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

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.iter())
    }
}

impl<'a, T: Data + Eq + Ord + 'a> DirectView<'a, T> for BTreeSetView<'a, T> {
    fn new(collector: Collector<'a, SetChange<T>>) -> Self {
        Self {
            collector,
            data: BTreeSet::new(),
        }
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
        collector: Collector<'a, SetChange<T>>,
        key_func: KF,
        value_func: VF,
    ) -> Self {
        Self {
            collector,
            data: HashMap::new(),
            key_func: Box::new(key_func),
            value_func: Box::new(value_func),
        }
    }

    pub fn unchanged_data(&self) -> &HashMap<K, V> {
        &self.data
    }

    pub fn data(&mut self) -> &HashMap<K, V> {
        self.update();
        self.unchanged_data()
    }
}

impl<'a, T: Data + 'a, K: Hash + Eq + 'a, V: 'a> View<'a, T> for HashMapView<'a, T, K, V> {
    type Item = V;

    fn update(&mut self) {
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

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.values())
    }
}

pub struct BTreeMapView<'a, T: Data + 'a, K: Ord + Eq + 'a, V: 'a> {
    collector: Collector<'a, SetChange<T>>,
    key_func: Box<dyn Fn(&T) -> K + 'a>,
    value_func: Box<dyn Fn(T) -> V + 'a>,
    data: BTreeMap<K, V>,
}

impl<'a, T: Data + 'a, K: Ord + Eq + 'a, V: 'a> BTreeMapView<'a, T, K, V> {
    pub fn new<KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        collector: Collector<'a, SetChange<T>>,
        key_func: KF,
        value_func: VF,
    ) -> Self {
        Self {
            collector,
            data: BTreeMap::new(),
            key_func: Box::new(key_func),
            value_func: Box::new(value_func),
        }
    }

    pub fn unchanged_data(&self) -> &BTreeMap<K, V> {
        &self.data
    }

    pub fn data(&mut self) -> &BTreeMap<K, V> {
        self.update();
        self.unchanged_data()
    }
}

impl<'a, T: Data + 'a, K: Ord + Eq + 'a, V: 'a> View<'a, T> for BTreeMapView<'a, T, K, V> {
    type Item = V;

    fn update(&mut self) {
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

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.values())
    }
}

pub struct VecIndexView<'a, T: Data + 'a, R: 'a> {
    collector: Collector<'a, SetChange<T>>,
    index_func: Box<dyn Fn(&T) -> usize + 'a>,
    value_func: Box<dyn Fn(T) -> R + 'a>,
    data: Vec<Option<R>>,
}

impl<'a, T: Data + 'a, R: 'a> VecIndexView<'a, T, R> {
    pub fn new<IF: Fn(&T) -> usize + 'a, VF: Fn(T) -> R + 'a>(
        collector: Collector<'a, SetChange<T>>,
        index_func: IF,
        value_func: VF,
    ) -> Self {
        Self {
            collector,
            data: Vec::new(),
            index_func: Box::new(index_func),
            value_func: Box::new(value_func),
        }
    }

    pub fn unchanged_data(&self) -> &Vec<Option<R>> {
        &self.data
    }

    pub fn data(&mut self) -> &Vec<Option<R>> {
        self.update();
        self.unchanged_data()
    }
}

impl<'a, T: Data + 'a, R: 'a> View<'a, T> for VecIndexView<'a, T, R> {
    type Item = Option<R>;

    fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;
        let idxf = &self.index_func;
        let vf = &self.value_func;

        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => {
                    let idx: usize = idxf(&t);
                    if store.len() <= idx {
                        store.resize_with(idx + 1, || None);
                    }
                    store[idx] = Some(vf(t));
                }
                SetChange::Removed(t) => {
                    store[idxf(&t)] = None;
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.iter())
    }
}

pub struct VecView<'a, T: Data + PartialEq + 'a> {
    data: Vec<T>,
    collector: Collector<'a, SetChange<T>>,
}

impl<'a, T: Data + PartialEq + 'a> VecView<'a, T> {
    pub fn unchanged_data(&self) -> &Vec<T> {
        &self.data
    }

    pub fn data(&mut self) -> &Vec<T> {
        self.update();
        self.unchanged_data()
    }
}

impl<'a, T: Data + PartialEq + 'a> View<'a, T> for VecView<'a, T> {
    type Item = T;

    fn update(&mut self) {
        self.collector.update();
        let store = &mut self.data;
        self.collector
            .items
            .drain(..)
            .for_each(|change| match change {
                SetChange::Added(t) => store.push(t),
                SetChange::Clear => store.clear(),
                SetChange::Removed(t) => {
                    if let Some(idx) = store.iter().position(|x| x == &t) {
                        store.remove(idx);
                    }
                }
            })
    }

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.iter())
    }
}

impl<'a, T: Data + PartialEq + 'a> DirectView<'a, T> for VecView<'a, T> {
    fn new(collector: Collector<'a, SetChange<T>>) -> Self {
        Self {
            collector,
            data: Vec::new(),
        }
    }
}

pub struct OrderedVecView<'a, T: Data + 'a, K: Copy + Eq + Ord + 'a> {
    data: Vec<(K, T)>,
    key_func: Box<dyn Fn(&T) -> K + 'a>,
    collector: Collector<'a, SetChange<T>>,
}

impl<'a, T: Data + 'a, K: Copy + Eq + Ord + 'a> OrderedVecView<'a, T, K> {
    pub fn new<KF: Fn(&T) -> K + 'a>(collector: Collector<'a, SetChange<T>>, key_func: KF) -> Self {
        OrderedVecView {
            data: Vec::new(),
            key_func: Box::new(key_func),
            collector,
        }
    }

    pub fn unchanged_data(&self) -> &Vec<(K, T)> {
        &self.data
    }

    pub fn data(&mut self) -> &Vec<(K, T)> {
        self.update();
        self.unchanged_data()
    }
}

impl<'a, T: Data + 'a, K: Copy + Eq + Ord + 'a> View<'a, T> for OrderedVecView<'a, T, K> {
    type Item = (K, T);

    fn update(&mut self) {
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
                    if let Ok(existing_idx) = store.binary_search_by_key(&key, |(k, _)| *k) {
                        store.remove(existing_idx);
                    } else {
                        //TODO: Panic?
                    }
                }
                SetChange::Clear => store.clear(),
            });
        self.collector.clear();
    }

    fn iter_unchanged<'b>(&'b self) -> Box<dyn Iterator<Item = &Self::Item> + 'b> {
        Box::new(self.data.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::*;

    #[test]
    fn test_hashset_view() {
        let world = World::new();
        let mut setp: CollectionPortal<i32> = CollectionPortal::new::<VecView<i32>>(&world);
        let mut view = setp.signal().view_set_hash();
        assert!(view.data_unchanged().is_empty());
        setp.add(2);
        view.update();
        assert!(view.data_unchanged().contains(&2));
        setp.remove(2);
        setp.add(3);
        view.update();
        assert!(!view.data_unchanged().contains(&2));
        setp.clear();
        view.update();
        assert!(view.data_unchanged().is_empty());
    }

    #[test]
    fn test_btreeset_view() {
        let world = World::new();
        let mut setp: CollectionPortal<i32> = CollectionPortal::new::<VecView<i32>>(&world);
        let mut view = setp.signal().view_set_btree();
        assert!(view.data_unchanged().is_empty());
        setp.add(2);
        view.update();
        assert!(view.data_unchanged().contains(&2));
        setp.remove(2);
        setp.add(3);
        view.update();
        assert!(!view.data_unchanged().contains(&2));
        setp.clear();
        view.update();
        assert!(view.data_unchanged().is_empty());
    }

    #[test]
    fn test_hashmap_view() {
        let world = World::new();
        let mut setp: CollectionPortal<(i32, i32)> =
            CollectionPortal::new::<VecView<(i32, i32)>>(&world);
        let mut view = setp.signal().view_map_hash(|(a, _)| *a, |(_, b)| b);
        assert!(view.data().is_empty());
        setp.add((2, 3));
        assert!(view.data().contains_key(&2));
        setp.remove((2, 3));
        setp.add((4, 3));
        assert!(!view.data().contains_key(&2));
        assert_eq!(3, view.data()[&4]);
        setp.add((4, 2));
        assert_eq!(2, view.data()[&4]);
        setp.clear();
        assert!(view.data().is_empty());
    }

    #[test]
    fn test_btreemap_view() {
        let world = World::new();
        let mut setp: CollectionPortal<(i32, i32)> =
            CollectionPortal::new::<VecView<(i32, i32)>>(&world);
        let mut view = setp.signal().view_map_btree(|(a, _)| *a, |(_, b)| b);
        assert!(view.data().is_empty());
        setp.add((2, 3));
        assert!(view.data().contains_key(&2));
        setp.remove((2, 3));
        setp.add((4, 3));
        assert!(!view.data().contains_key(&2));
        assert_eq!(3, view.data()[&4]);
        setp.add((4, 2));
        assert_eq!(2, view.data()[&4]);
        setp.clear();
        assert!(view.data().is_empty());
    }

    #[test]
    fn test_vec_indexed() {
        let world = World::new();
        let mut setp: CollectionPortal<(i32, i32)> =
            CollectionPortal::new::<VecView<(i32, i32)>>(&world);
        let mut view = setp
            .signal()
            .view_vec_indexed(|(a, _)| *a as usize, |(_, b)| b);
        setp.add((0, 2));
        setp.add((1, 3));
        setp.add((3, 2));
        assert_eq!(view.data(), &vec![Some(2), Some(3), None, Some(2)]);
        setp.remove((1, 3));
        assert_eq!(view.data(), &vec![Some(2), None, None, Some(2)]);
        setp.add((0, 4));
        assert_eq!(view.data(), &vec![Some(4), None, None, Some(2)]);
        setp.clear();
        assert!(view.data().is_empty());
    }

    fn view_values(view: &mut OrderedVecView<i32, i32>) -> Vec<i32> {
        view.iter().map(|(_, b)| *b).collect()
    }

    #[test]
    fn test_vec_ordered() {
        let world = World::new();
        let mut setp: CollectionPortal<i32> = CollectionPortal::new::<VecView<i32>>(&world);
        let mut view = setp.signal().view_vec_sorted(|i| *i);
        setp.add(2);
        setp.add(3);
        setp.add(1);
        assert_eq!(view_values(&mut view), vec![1, 2, 3]);
        setp.remove(2);
        assert_eq!(view_values(&mut view), vec![1, 3]);
        setp.add(4);
        assert_eq!(view_values(&mut view), vec![1, 3, 4]);
        setp.add(2);
        assert_eq!(view_values(&mut view), vec![1, 2, 3, 4]);
        setp.clear();
        assert!(view.data().is_empty());
    }

    #[test]
    fn test_vec_ordered_replace() {
        let world = World::new();
        let mut setp: CollectionPortal<i32> = CollectionPortal::new::<VecView<i32>>(&world);
        let mut view = setp.signal().view_vec_sorted(|i| *i / 2);
        setp.add(0);
        setp.add(1);
        assert_eq!(view_values(&mut view), vec![1]);
        setp.remove(1);
        assert!(view.data().is_empty());
        setp.add(2);
        setp.add(4);
        setp.add(7);
        assert_eq!(view_values(&mut view), vec![2, 4, 7]);
    }

    #[test]
    fn test_vec_view() {
        let world = World::new();
        let mut setp: CollectionPortal<i32> = CollectionPortal::new::<VecView<i32>>(&world);
        let mut view = setp.signal().view_vec();
        setp.add(0);
        setp.add(1);
        assert_eq!(view.data(), &vec![0, 1]);
        setp.add(0);
        assert_eq!(view.data(), &vec![0, 1, 0]);
        setp.remove(0);
        assert_eq!(view.data(), &vec![1, 0]);
        setp.clear();
        assert!(view.data().is_empty())
    }

    #[test]
    fn test_later_attachment_1() {
        let world = World::new();
        let mut setp: CollectionPortal<i32> = CollectionPortal::new::<VecView<i32>>(&world);
        let mut view1 = setp.signal().view_vec();
        setp.add(0);
        setp.add(1);
        assert_eq!(view1.data(), &vec![0, 1]);
        let mut view2 = setp.signal().view_vec();
        assert_eq!(view2.data(), &vec![0, 1]);
        setp.add(3);
        assert_eq!(view1.data(), &vec![0, 1, 3]);
        assert_eq!(view2.data(), &vec![0, 1, 3]);
    }
}
