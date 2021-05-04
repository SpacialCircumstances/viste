use crate::graph::{Graph, NodeIndex, SearchContinuation};
use crate::streams::last::Last;
use crate::streams::portal::Portal;
use crate::values::binder::Binder;
use crate::values::constant::Constant;
use crate::values::filter::Filter;
use crate::values::mapper::{Mapper, Mapper2};
use crate::values::mutable::Mutable;
use log::info;
use slab::Slab;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

mod graph;
mod streams;
mod values;

pub trait Data: Debug {
    fn changed(&self, other: &Self) -> bool;
    fn cheap_clone(&self) -> Self;
}

//TODO: Find a way to only impl for Rc, Arc, T: Copy
impl<T: Debug + Clone + PartialEq> Data for T {
    fn changed(&self, other: &T) -> bool {
        self != other
    }

    fn cheap_clone(&self) -> Self {
        self.clone()
    }
}

struct WorldData {
    dependencies: Graph<bool>,
}

pub struct World(Rc<RefCell<WorldData>>);

impl World {
    pub fn new() -> Self {
        World(Rc::new(RefCell::new(WorldData {
            dependencies: Graph::new(),
        })))
    }

    pub fn mark_dirty(&self, node: NodeIndex) {
        let mut wd = self.0.borrow_mut();
        let old_dirty = wd.dependencies[node];
        if !old_dirty {
            wd.dependencies.search_children_mut(
                |child| {
                    if !*child {
                        *child = true;
                        SearchContinuation::Continue
                    } else {
                        SearchContinuation::Stop
                    }
                },
                node,
            );
        }
    }

    pub fn is_dirty(&self, node: NodeIndex) -> bool {
        let wd = self.0.borrow();
        wd.dependencies[node]
    }

    pub fn unmark(&self, node: NodeIndex) {
        self.0.borrow_mut().dependencies[node] = false;
    }

    pub fn create_node(&self) -> NodeIndex {
        self.0.borrow_mut().dependencies.add_node(true)
    }

    pub fn destroy_node(&self, node: NodeIndex) {
        self.0.borrow_mut().dependencies.remove_node(node);
    }

    pub fn add_dependency(&self, parent: NodeIndex, child: NodeIndex) {
        self.0.borrow_mut().dependencies.add_edge(parent, child);
    }

    pub fn remove_dependency(&self, parent: NodeIndex, child: NodeIndex) {
        let mut wd = self.0.borrow_mut();
        wd.dependencies.remove_edge(parent, child);
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for World {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct ReaderToken(usize);

#[derive(Debug, Eq, PartialEq)]
pub enum SingleComputationResult<T: Data> {
    Changed(T),
    Unchanged,
}

impl<T: Data> SingleComputationResult<T> {
    pub fn unwrap_changed(self) -> T {
        match self {
            SingleComputationResult::Changed(v) => v,
            SingleComputationResult::Unchanged => {
                panic!("Tried to unwrap changed value, but was unchanged")
            }
        }
    }
}

pub trait ComputationCore {
    type ComputationResult;
    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult;
    fn create_reader(&mut self) -> ReaderToken;
    fn destroy_reader(&mut self, reader: ReaderToken);
    fn add_dependency(&mut self, child: NodeIndex);
    fn remove_dependency(&mut self, child: NodeIndex);
    fn is_dirty(&self) -> bool;
    fn world(&self) -> &World;
}

pub trait Signal<'a, S: 'a> {
    fn create<C: ComputationCore<ComputationResult = S> + 'a>(r: C) -> Self;
    fn world(&self) -> World;
    fn compute(&self, reader: ReaderToken) -> S;
    fn add_dependency(&self, child: NodeIndex);
    fn remove_dependency(&self, child: NodeIndex);
    fn create_reader(&self) -> ReaderToken;
    fn destroy_reader(&self, reader: ReaderToken);
    fn is_dirty(&self) -> bool;
}

pub struct StreamSignal<'a, T: Data + 'a>(
    Rc<RefCell<dyn ComputationCore<ComputationResult = Option<T>> + 'a>>,
);

impl<'a, T: Data + 'a> Clone for StreamSignal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T: Data + 'a> Signal<'a, Option<T>> for StreamSignal<'a, T> {
    fn create<S: ComputationCore<ComputationResult = Option<T>> + 'a>(r: S) -> Self {
        Self(Rc::new(RefCell::new(r)))
    }

    fn world(&self) -> World {
        self.0.borrow().world().clone()
    }

    fn compute(&self, reader: ReaderToken) -> Option<T> {
        self.0.borrow_mut().compute(reader)
    }

    fn add_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().add_dependency(child)
    }

    fn remove_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().remove_dependency(child)
    }

    fn create_reader(&self) -> ReaderToken {
        self.0.borrow_mut().create_reader()
    }

    fn destroy_reader(&self, reader: ReaderToken) {
        self.0.borrow_mut().destroy_reader(reader)
    }

    fn is_dirty(&self) -> bool {
        self.0.borrow().is_dirty()
    }
}

impl<'a, T: Data + 'a> StreamSignal<'a, T> {
    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> StreamSignal<'a, R> {
        StreamSignal::create(streams::mapper::Mapper::new(
            self.world(),
            self.clone(),
            mapper,
        ))
    }

    pub fn last(&self, initial: T) -> ValueSignal<'a, T> {
        ValueSignal::create(Last::new(self.world(), self.clone(), initial))
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F) -> StreamSignal<'a, T> {
        StreamSignal::create(streams::filter::Filter::new(
            self.world(),
            self.clone(),
            filter,
        ))
    }

    pub fn cached(&self) -> StreamSignal<'a, T> {
        StreamSignal::create(streams::cached::Cached::new(self.world(), self.clone()))
    }

    pub fn collect(&self) -> Collector<'a, T> {
        Collector::new(StreamReader::new(self.clone()))
    }
}

pub struct ValueSignal<'a, T: Data>(
    Rc<RefCell<dyn ComputationCore<ComputationResult = SingleComputationResult<T>> + 'a>>,
);

impl<'a, T: Data + 'a> Signal<'a, SingleComputationResult<T>> for ValueSignal<'a, T> {
    fn create<S: ComputationCore<ComputationResult = SingleComputationResult<T>> + 'a>(
        r: S,
    ) -> Self {
        Self(Rc::new(RefCell::new(r)))
    }

    fn world(&self) -> World {
        self.0.borrow().world().clone()
    }

    fn compute(&self, reader: ReaderToken) -> SingleComputationResult<T> {
        self.0.borrow_mut().compute(reader)
    }

    fn add_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().add_dependency(child)
    }

    fn remove_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().remove_dependency(child)
    }

    fn create_reader(&self) -> ReaderToken {
        self.0.borrow_mut().create_reader()
    }

    fn destroy_reader(&self, reader: ReaderToken) {
        self.0.borrow_mut().destroy_reader(reader)
    }

    fn is_dirty(&self) -> bool {
        self.0.borrow().is_dirty()
    }
}

impl<'a, T: Data + 'a> ValueSignal<'a, T> {
    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> ValueSignal<'a, R> {
        ValueSignal::create(Mapper::new(self.world(), self.clone(), mapper))
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(self, filter: F, initial: T) -> ValueSignal<'a, T> {
        ValueSignal::create(Filter::new(self.world(), self.clone(), initial, filter))
    }

    pub fn bind<O: Data + 'a, B: Fn(T) -> ValueSignal<'a, O> + 'a>(
        &self,
        binder: B,
    ) -> ValueSignal<'a, O> {
        ValueSignal::create(Binder::new(self.world(), self.clone(), binder))
    }

    pub fn changed(&self) -> StreamSignal<'a, T> {
        StreamSignal::create(streams::changed::Changed::new(self.world(), self.clone()))
    }
}

impl<'a, T: Data + 'a> Debug for ValueSignal<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dirty = self.is_dirty();
        let value = read_once(self);
        write!(f, "Signal {{ dirty: {}, value: {:?} }}", dirty, value)
    }
}

pub fn portal<'a, T: Data + 'a>(world: &World) -> (impl Fn(T), StreamSignal<'a, T>) {
    let p = Portal::new(world.clone());
    let signal = Rc::new(RefCell::new(p));
    let s = signal.clone();
    let pusher = move |v| s.borrow_mut().send(v);
    (pusher, StreamSignal(signal))
}

pub fn mutable<'a, T: Data + 'a>(world: &World, initial: T) -> (impl Fn(T), ValueSignal<'a, T>) {
    let m = Mutable::new(world.clone(), initial);
    let signal = Rc::new(RefCell::new(m));
    let s = signal.clone();
    let mutator = move |v| s.borrow_mut().set(v);
    (mutator, ValueSignal(signal))
}

pub fn constant<'a, T: Data + 'a>(world: &World, value: T) -> ValueSignal<'a, T> {
    ValueSignal::create(Constant::new(world.clone(), value))
}

pub fn map2<'a, T1: Data + 'a, T2: Data + 'a, O: Data + 'a, M: Fn(T1, T2) -> O + 'a>(
    s1: &ValueSignal<'a, T1>,
    s2: &ValueSignal<'a, T2>,
    mapper: M,
) -> ValueSignal<'a, O> {
    ValueSignal::create(Mapper2::new(s1.world(), s1.clone(), s2.clone(), mapper))
}

impl<'a, T: Data> Clone for ValueSignal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct ParentValueSignal<
    'a,
    T: Data + 'a,
    S,
    R: Reader<Result = S, Signal = ValueSignal<'a, T>>,
> {
    parent: ValueSignal<'a, T>,
    own_index: NodeIndex,
    reader: R,
}

impl<'a, T: Data + 'a, S, R: Reader<Result = S, Signal = ValueSignal<'a, T>>>
    ParentValueSignal<'a, T, S, R>
{
    pub fn new(signal: ValueSignal<'a, T>, own_index: NodeIndex) -> Self {
        signal.add_dependency(own_index);
        let reader = R::new(signal.clone());
        Self {
            parent: signal,
            own_index,
            reader,
        }
    }

    pub fn set_parent(&mut self, signal: ValueSignal<'a, T>) {
        self.parent.remove_dependency(self.own_index);
        signal.add_dependency(self.own_index);
        self.parent = signal;
    }

    pub fn compute(&mut self) -> S {
        self.reader.read()
    }
}

impl<'a, T: Data + 'a, S, R: Reader<Result = S, Signal = ValueSignal<'a, T>>> Drop
    for ParentValueSignal<'a, T, S, R>
{
    fn drop(&mut self) {
        info!("Removing {} from parent", self.own_index);
        self.parent.remove_dependency(self.own_index);
    }
}

pub struct OwnNode(World, NodeIndex);

impl OwnNode {
    pub fn new(world: World) -> Self {
        let idx = world.create_node();
        Self(world, idx)
    }

    pub fn world(&self) -> &World {
        &self.0
    }

    pub fn node(&self) -> NodeIndex {
        self.1
    }

    pub fn add_dependency(&self, to: NodeIndex) {
        self.0.add_dependency(self.1, to)
    }

    pub fn remove_dependency(&self, to: NodeIndex) {
        self.0.remove_dependency(self.1, to)
    }

    pub fn is_dirty(&self) -> bool {
        self.0.is_dirty(self.1)
    }

    pub fn clean(&self) {
        self.0.unmark(self.1)
    }

    pub fn mark_dirty(&self) {
        self.0.mark_dirty(self.1)
    }
}

impl Drop for OwnNode {
    fn drop(&mut self) {
        info!("Dropping signal: {}", self.1);
        self.0.destroy_node(self.1)
    }
}

pub struct SingleValueStore<T: Data> {
    value: T,
    reader_states: Slab<bool>,
}

impl<T: Data> SingleValueStore<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            reader_states: Slab::new(),
        }
    }

    pub fn create_reader(&mut self) -> ReaderToken {
        let reader = self.reader_states.insert(false);
        ReaderToken(reader)
    }

    pub fn destroy_reader(&mut self, reader: ReaderToken) {
        self.reader_states.remove(reader.0);
    }

    pub fn set_value(&mut self, value: T) {
        self.value = value;
        self.reader_states
            .iter_mut()
            .for_each(|(_, rs)| *rs = false);
    }

    pub fn read(&mut self, reader: ReaderToken) -> SingleComputationResult<T> {
        let state = self
            .reader_states
            .get_mut(reader.0)
            .expect("Reader not found");
        if !*state {
            *state = true;
            SingleComputationResult::Changed(self.value.cheap_clone())
        } else {
            SingleComputationResult::Unchanged
        }
    }
}

pub struct BufferedStore<T: Data> {
    //TODO: Optimize with single queue and position as state
    reader_states: Slab<VecDeque<T>>,
}

impl<T: Data> BufferedStore<T> {
    pub fn new() -> Self {
        Self {
            reader_states: Slab::new(),
        }
    }

    pub fn create_reader(&mut self) -> ReaderToken {
        ReaderToken(self.reader_states.insert(VecDeque::new()))
    }

    pub fn destroy_reader(&mut self, reader: ReaderToken) {
        self.reader_states.remove(reader.0);
    }

    pub fn read(&mut self, reader: ReaderToken) -> Option<T> {
        self.reader_states[reader.0].pop_front()
    }

    pub fn push(&mut self, value: T) {
        self.reader_states
            .iter_mut()
            .for_each(|(_, rs)| rs.push_back(value.cheap_clone()))
    }
}

fn read_once<'a, T: Data + 'a>(signal: &ValueSignal<'a, T>) -> T {
    let reader = signal.create_reader();
    let value = signal.compute(reader);
    signal.destroy_reader(reader);
    value.unwrap_changed()
}

pub trait Reader {
    type Result;
    type Signal;
    fn new(signal: Self::Signal) -> Self;
    fn read(&mut self) -> Self::Result;
}

pub struct ChangeReader<'a, T: Data + 'a>(ValueSignal<'a, T>, ReaderToken);

impl<'a, T: Data + 'a> Reader for ChangeReader<'a, T> {
    type Result = SingleComputationResult<T>;
    type Signal = ValueSignal<'a, T>;

    fn new(signal: ValueSignal<'a, T>) -> Self {
        let reader = signal.create_reader();
        Self(signal, reader)
    }

    fn read(&mut self) -> Self::Result {
        self.0.compute(self.1)
    }
}

impl<'a, T: Data + 'a> Drop for ChangeReader<'a, T> {
    fn drop(&mut self) {
        self.0.destroy_reader(self.1)
    }
}

pub struct CachedReader<'a, T: Data + 'a> {
    signal: ValueSignal<'a, T>,
    token: ReaderToken,
    cache: T,
}

impl<'a, T: Data + 'a> Reader for CachedReader<'a, T> {
    type Result = (bool, T);
    type Signal = ValueSignal<'a, T>;

    fn new(signal: ValueSignal<'a, T>) -> Self {
        let token = signal.create_reader();
        let initial_value = signal.compute(token).unwrap_changed();
        Self {
            signal,
            token,
            cache: initial_value,
        }
    }

    fn read(&mut self) -> (bool, T) {
        match self.signal.compute(self.token) {
            SingleComputationResult::Changed(new_v) => {
                self.cache = new_v;
                (true, self.cache.cheap_clone())
            }
            SingleComputationResult::Unchanged => (false, self.cache.cheap_clone()),
        }
    }
}

impl<'a, T: Data + 'a> Drop for CachedReader<'a, T> {
    fn drop(&mut self) {
        self.signal.destroy_reader(self.token)
    }
}

pub struct StreamReader<'a, T: Data + 'a> {
    signal: StreamSignal<'a, T>,
    token: ReaderToken,
}

pub struct StreamReaderIter<'a, T: Data + 'a>(StreamReader<'a, T>);

impl<'a, T: Data + 'a> Iterator for StreamReaderIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.read()
    }
}

impl<'a, T: Data + 'a> Reader for StreamReader<'a, T> {
    type Result = Option<T>;
    type Signal = StreamSignal<'a, T>;

    fn new(signal: Self::Signal) -> Self {
        let token = signal.create_reader();
        Self { signal, token }
    }

    fn read(&mut self) -> Self::Result {
        self.signal.compute(self.token)
    }
}

impl<'a, T: Data + 'a> IntoIterator for StreamReader<'a, T> {
    type Item = T;
    type IntoIter = StreamReaderIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        StreamReaderIter(self)
    }
}

impl<'a, T: Data + 'a> Drop for StreamReader<'a, T> {
    fn drop(&mut self) {
        self.signal.destroy_reader(self.token)
    }
}

pub struct ParentStreamSignal<
    'a,
    T: Data + 'a,
    S,
    R: Reader<Result = S, Signal = StreamSignal<'a, T>>,
> {
    parent: StreamSignal<'a, T>,
    own_index: NodeIndex,
    reader: R,
}

impl<'a, T: Data + 'a, S, R: Reader<Result = S, Signal = StreamSignal<'a, T>>>
    ParentStreamSignal<'a, T, S, R>
{
    pub fn new(signal: StreamSignal<'a, T>, own_index: NodeIndex) -> Self {
        signal.add_dependency(own_index);
        let reader = R::new(signal.clone());
        Self {
            parent: signal,
            own_index,
            reader,
        }
    }

    pub fn set_parent(&mut self, signal: StreamSignal<'a, T>) {
        self.parent.remove_dependency(self.own_index);
        signal.add_dependency(self.own_index);
        self.parent = signal;
    }

    pub fn compute(&mut self) -> S {
        self.reader.read()
    }
}

impl<'a, T: Data + 'a, S, R: Reader<Result = S, Signal = StreamSignal<'a, T>>> Drop
    for ParentStreamSignal<'a, T, S, R>
{
    fn drop(&mut self) {
        info!("Removing {} from parent", self.own_index);
        self.parent.remove_dependency(self.own_index);
    }
}

pub struct Collector<'a, T: Data + 'a> {
    reader: StreamReader<'a, T>,
    items: Vec<T>,
}

impl<'a, T: Data + 'a> Collector<'a, T> {
    pub fn new(reader: StreamReader<'a, T>) -> Self {
        Self {
            reader,
            items: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        while let Some(next) = self.reader.read() {
            self.items.push(next);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_map() {
        let world = World::new();
        let (set, v) = mutable(&world, 3);
        let map1 = v.map(|x| x + 1);
        let map2 = v.map(|x| x + 2);
        assert_eq!(4, read_once(&map1));
        assert_eq!(5, read_once(&map2));
        set(6);
        assert_eq!(7, read_once(&map1));
        assert_eq!(8, read_once(&map2));
    }

    #[test]
    fn test_map2() {
        let world = World::new();
        let (set1, v1) = mutable(&world, 0);
        let (set2, v2) = mutable(&world, 0);
        let mapped = map2(&v1, &v2, |x, y| x + y);
        assert_eq!(0, read_once(&mapped));
        set1(1);
        assert_eq!(1, read_once(&mapped));
        set2(1);
        assert_eq!(2, read_once(&mapped));
        set1(3);
        set2(0);
        assert_eq!(3, read_once(&mapped));
        assert_eq!(3, read_once(&mapped));
    }

    #[test]
    fn test_map2_constant() {
        let world = World::new();
        let (set1, v1) = mutable(&world, 0);
        let v2 = constant(&world, 1);
        let mapped = map2(&v1, &v2, |x, y| x + y);
        assert_eq!(1, read_once(&mapped));
        set1(1);
        assert_eq!(2, read_once(&mapped));
        set1(3);
        assert_eq!(4, read_once(&mapped));
        assert_eq!(4, read_once(&mapped));
        set1(0);
        assert_eq!(1, read_once(&mapped));
    }

    #[test]
    fn test_filter() {
        let world = World::new();
        let (set, v) = mutable(&world, 0);
        let filtered = v.filter(|x| x % 2 == 0, 0);
        assert_eq!(0, read_once(&filtered));
        set(1);
        assert_eq!(0, read_once(&filtered));
        set(2);
        assert_eq!(2, read_once(&filtered));
        set(3);
        assert_eq!(2, read_once(&filtered));
        set(0);
        assert_eq!(0, read_once(&filtered));
    }

    #[test]
    fn test_bind() {
        let world = World::new();
        let c = constant(&world, 2);
        let (set, v) = mutable(&world, 0);
        let bound = v.bind(move |x| c.map(move |v| v + x));
        assert_eq!(2, read_once(&bound));
        set(2);
        assert_eq!(4, read_once(&bound));
        set(3);
        assert_eq!(5, read_once(&bound));
    }

    #[test]
    fn test_bind2() {
        let world = World::new();
        let (set1, v1) = mutable(&world, 1);
        let (set2, v2) = mutable(&world, 2);
        let (switch, switcher) = mutable(&world, false);
        let b = switcher.bind(move |b| if b { v1.clone() } else { v2.clone() });
        assert_eq!(2, read_once(&b));
        switch(true);
        assert_eq!(1, read_once(&b));
        set1(4);
        assert_eq!(4, read_once(&b));
        set2(6);
        assert_eq!(4, read_once(&b));
        switch(false);
        assert_eq!(6, read_once(&b));
        switch(true);
        assert_eq!(4, read_once(&b));
    }

    #[test]
    fn test_stream() {
        let world = World::new();
        let (send, s1) = portal(&world);
        let res = s1.last(0);
        assert_eq!(0, read_once(&res));
        send(1);
        assert_eq!(1, read_once(&res));
        send(2);
        send(3);
        assert_eq!(3, read_once(&res));
    }

    fn collect_all<T: Data>(coll: &Collector<T>) -> Vec<T> {
        coll.iter().map(|t| t.cheap_clone()).collect()
    }

    #[test]
    fn test_collect() {
        let world = World::new();
        let (send, s1) = portal(&world);
        let mut collector = s1.collect();
        assert_eq!(collect_all(&collector), Vec::new());
        send(1);
        assert_eq!(collect_all(&collector), Vec::new());
        collector.update();
        assert_eq!(collect_all(&collector), vec![1]);
        send(2);
        send(3);
        send(4);
        collector.update();
        assert_eq!(collect_all(&collector), vec![1, 2, 3, 4]);
    }
}
