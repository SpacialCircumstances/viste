use crate::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

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

pub trait View<'a, 'b, T: Data + 'a> {
    type Item;
    fn update(&mut self);
    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b>;
    fn iter(&'b mut self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
        self.update();
        self.iter_unchanged()
    }
}

pub trait DirectView<'a, T: Data + 'a>: View<'a, 'a, T, Item = &'a T> {
    fn new(collector: Collector<'a, SetChange<T>>) -> Self;
}

#[derive(Eq, PartialEq)]
pub struct SharedView<'a, 'b, T: Data + 'a + 'b, V: View<'a, 'b, T>>(
    Rc<RefCell<V>>,
    PhantomData<&'a T>,
    PhantomData<&'b T>,
);

impl<'a, 'b, T: Data + 'a + 'b, V: View<'a, 'b, T>> Clone for SharedView<'a, 'b, T, V> {
    fn clone(&self) -> Self {
        Self(
            self.0.clone(),
            PhantomData::default(),
            PhantomData::default(),
        )
    }
}

impl<'a, 'b, T: Data + 'a, V: View<'a, 'b, T>> SharedView<'a, 'b, T, V> {
    pub fn new(view: V) -> Self {
        Self(
            Rc::new(RefCell::new(view)),
            PhantomData::default(),
            PhantomData::default(),
        )
    }
}

#[derive(PartialEq, Eq)]
pub struct CollectionSignal<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> {
    stream: StreamSignal<'a, SetChange<T>>,
    view: SharedView<'a, 'a, T, D>,
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> Clone for CollectionSignal<'a, T, D> {
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            view: self.view.clone(),
        }
    }
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> Signal<'a, Option<SetChange<T>>>
    for CollectionSignal<'a, T, D>
{
    fn create<C: ComputationCore<ComputationResult = Option<SetChange<T>>> + 'a>(r: C) -> Self {
        Self::new(StreamSignal::create(r))
    }

    fn world(&self) -> World {
        self.stream.world()
    }

    fn compute(&self, reader: ReaderToken) -> Option<SetChange<T>> {
        self.stream.compute(reader)
    }

    fn add_dependency(&self, child: NodeIndex) {
        self.stream.add_dependency(child)
    }

    fn remove_dependency(&self, child: NodeIndex) {
        self.stream.remove_dependency(child)
    }

    fn create_reader(&self) -> ReaderToken {
        self.stream.create_reader()
    }

    fn destroy_reader(&self, reader: ReaderToken) {
        self.stream.destroy_reader(reader)
    }

    fn is_dirty(&self) -> bool {
        self.stream.is_dirty()
    }

    fn node(&self) -> NodeIndex {
        self.stream.node()
    }
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> From<StreamSignal<'a, SetChange<T>>>
    for CollectionSignal<'a, T, D>
{
    fn from(stream: StreamSignal<'a, SetChange<T>>) -> Self {
        CollectionSignal::new(stream)
    }
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> CollectionSignal<'a, T, D> {
    pub fn new(stream: StreamSignal<'a, SetChange<T>>) -> Self {
        let view = D::new(stream.collect());
        Self {
            stream,
            view: SharedView::new(view),
        }
    }

    pub fn changes(&self) -> StreamSignal<'a, SetChange<T>> {
        self.stream.clone()
    }

    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a, D2: DirectView<'a, R>>(
        &self,
        mapper: M,
    ) -> CollectionSignal<'a, R, D2> {
        self.stream
            .map(move |c| match c {
                SetChange::Added(t) => SetChange::Added(mapper(t)),
                SetChange::Removed(t) => SetChange::Removed(mapper(t)),
                SetChange::Clear => SetChange::Clear,
            })
            .into()
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F) -> CollectionSignal<'a, T, D> {
        self.stream
            .filter(move |c| match c {
                SetChange::Added(t) => filter(t),
                SetChange::Removed(t) => filter(t),
                SetChange::Clear => true,
            })
            .into()
    }

    pub fn filter_map<O: Data + 'a, F: Fn(T) -> Option<O> + 'a, D2: DirectView<'a, O>>(
        &self,
        f: F,
    ) -> CollectionSignal<'a, O, D2> {
        self.stream
            .filter_map(move |c| match c {
                SetChange::Added(t) => f(t).map(SetChange::Added),
                SetChange::Removed(t) => f(t).map(SetChange::Removed),
                SetChange::Clear => Some(SetChange::Clear),
            })
            .into()
    }

    pub fn view_set_hash(&self) -> HashSetView<'a, T>
    where
        T: Hash + Eq,
    {
        HashSetView::new(self.stream.collect())
    }

    pub fn view_set_btree(&self) -> BTreeSetView<'a, T>
    where
        T: Ord + Eq,
    {
        BTreeSetView::new(self.stream.collect())
    }

    pub fn view_map_hash<K: Hash + Eq + 'a, V: 'a, KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        &self,
        key_func: KF,
        value_func: VF,
    ) -> HashMapView<'a, T, K, V> {
        HashMapView::new(self.stream.collect(), key_func, value_func)
    }

    pub fn view_map_btree<K: Ord + Eq + 'a, V: 'a, KF: Fn(&T) -> K + 'a, VF: Fn(T) -> V + 'a>(
        &self,
        key_func: KF,
        value_func: VF,
    ) -> BTreeMapView<'a, T, K, V> {
        BTreeMapView::new(self.stream.collect(), key_func, value_func)
    }

    pub fn view_vec_indexed<R: 'a, IF: Fn(&T) -> usize + 'a, VF: Fn(T) -> R + 'a>(
        &self,
        index_func: IF,
        value_func: VF,
    ) -> VecIndexView<'a, T, R> {
        VecIndexView::new(self.stream.collect(), index_func, value_func)
    }

    pub fn view_vec_sorted<K: Copy + Ord + Eq + Data, KF: Fn(&T) -> K + 'a>(
        &self,
        key_func: KF,
    ) -> OrderedVecView<'a, T, K> {
        OrderedVecView::new(self.stream.collect(), key_func)
    }

    pub fn view_vec(&self) -> VecView<'a, T>
    where
        T: PartialEq,
    {
        VecView::new(self.stream.collect())
    }
}

pub struct CollectionPortal<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> {
    signal: CollectionSignal<'a, T, D>,
    sender: Box<dyn Fn(SetChange<T>) + 'a>,
}

impl<'a, T: Data + 'a, D: DirectView<'a, T> + 'a> CollectionPortal<'a, T, D> {
    pub fn new(world: &World) -> Self {
        let (sender, signal) = portal(world);
        CollectionPortal {
            sender: Box::new(sender),
            signal: CollectionSignal::new(signal),
        }
    }

    pub fn signal(&self) -> &CollectionSignal<'a, T, D> {
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

impl<'a: 'b, 'b, T: Data + Hash + Eq + 'a> View<'a, 'b, T> for HashSetView<'a, T> {
    type Item = &'b T;

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
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

impl<'a: 'b, 'b, T: Data + Eq + Ord + 'a> View<'a, 'b, T> for BTreeSetView<'a, T> {
    type Item = &'b T;

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
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

impl<'a: 'b, 'b, T: Data + 'a, K: Hash + Eq + 'a, V: 'a> View<'a, 'b, T>
    for HashMapView<'a, T, K, V>
{
    type Item = (&'b K, &'b V);

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
        Box::new(self.data.iter())
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

impl<'a: 'b, 'b, T: Data + 'a, K: Ord + Eq + 'a, V: 'a> View<'a, 'b, T>
    for BTreeMapView<'a, T, K, V>
{
    type Item = (&'b K, &'b V);

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
        Box::new(self.data.iter())
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

impl<'a: 'b, 'b, T: Data + 'a, R: 'a> View<'a, 'b, T> for VecIndexView<'a, T, R> {
    type Item = &'b Option<R>;

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
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

impl<'a: 'b, 'b, T: Data + PartialEq + 'a> View<'a, 'b, T> for VecView<'a, T> {
    type Item = &'b T;

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
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

impl<'a: 'b, 'b, T: Data + 'a, K: Copy + Eq + Ord + 'a> View<'a, 'b, T>
    for OrderedVecView<'a, T, K>
{
    type Item = &'b (K, T);

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

    fn iter_unchanged(&'b self) -> Box<dyn Iterator<Item = Self::Item> + 'b> {
        Box::new(self.data.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::*;

    #[test]
    fn test_hashset_view() {
        let world = World::new();
        let mut setp: CollectionPortal<i32, VecView<i32>> = CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<i32, VecView<i32>> = CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<(i32, i32), VecView<(i32, i32)>> =
            CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<(i32, i32), VecView<(i32, i32)>> =
            CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<(i32, i32), VecView<(i32, i32)>> =
            CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<i32, VecView<i32>> = CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<i32, VecView<i32>> = CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<i32, VecView<i32>> = CollectionPortal::new(&world);
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
        let mut setp: CollectionPortal<i32, VecView<i32>> = CollectionPortal::new(&world);
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
