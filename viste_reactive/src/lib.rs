use crate::graph::{Graph, NodeIndex, SearchContinuation};
use crate::readers::{Reader, StreamReader};
use crate::streams::combine_mapper::CombineMapper;
use crate::streams::from_iter::FromIter;
use crate::streams::last::Last;
use crate::streams::many::Many;
use crate::streams::portal::Portal;
use crate::streams::zip_mapper::ZipMapper;
use crate::values::binder::{Binder, Binder2};
use crate::values::constant::Constant;
use crate::values::filter::Filter;
use crate::values::folder::Folder;
use crate::values::mapper::{Mapper, Mapper2};
use crate::values::mutable::Mutable;
use log::info;
use slab::Slab;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::mem::replace;
use std::rc::Rc;

pub mod collections;
pub mod graph;
pub mod readers;
pub mod stores;
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

#[derive(Debug)]
pub struct Distinct<T: Data>(T);

impl<T: Data> Distinct<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }

    pub fn deconstruct(self) -> T {
        self.0
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}

impl<T: Data> Data for Distinct<T> {
    fn changed(&self, _other: &Self) -> bool {
        true
    }

    fn cheap_clone(&self) -> Self {
        Distinct(self.0.cheap_clone())
    }
}

pub enum DirtyFlag {
    Basic(bool),
    Changed(Vec<NodeIndex>), //TODO: Investigate smallvec
}

impl DirtyFlag {
    fn clean() -> Self {
        DirtyFlag::Basic(false)
    }

    fn dirty() -> Self {
        DirtyFlag::Basic(true)
    }

    fn is_dirty(&self) -> bool {
        match self {
            DirtyFlag::Basic(b) => *b,
            DirtyFlag::Changed(nodes) => !nodes.is_empty(),
        }
    }

    fn unmark(&mut self) {
        match self {
            DirtyFlag::Basic(_) => *self = DirtyFlag::clean(),
            DirtyFlag::Changed(changed) => changed.clear(),
        }
    }

    fn mark(&mut self, cause: DirtyingCause) {
        match (self, cause) {
            (DirtyFlag::Changed(changed), DirtyingCause::Parent(p)) => changed.push(p),
            (d, DirtyingCause::Parent(p)) => {
                if let DirtyFlag::Basic(false) = d {
                    *d = DirtyFlag::Changed(vec![p])
                } // Else we need to recalculate all parents anyways, unfortunately
            }
            (d, DirtyingCause::External) => *d = DirtyFlag::dirty(),
        }
    }
}

impl Default for DirtyFlag {
    fn default() -> Self {
        Self::dirty()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DirtyingCause {
    External,
    Parent(NodeIndex),
}

struct WorldData {
    dependencies: Graph<DirtyFlag>,
}

pub struct World(Rc<RefCell<WorldData>>);

impl World {
    pub fn new() -> Self {
        World(Rc::new(RefCell::new(WorldData {
            dependencies: Graph::new(),
        })))
    }

    pub fn mark_dirty(&self, node: NodeIndex, cause: DirtyingCause) {
        let mut wd = self.0.borrow_mut();
        let old_dirty = &wd.dependencies[node];
        if !old_dirty.is_dirty() {
            wd.dependencies.search_children_mut(
                |child, child_idx, state| {
                    if !child.is_dirty() {
                        child.mark(state);
                        SearchContinuation::Continue(DirtyingCause::Parent(child_idx))
                    } else {
                        SearchContinuation::Stop
                    }
                },
                node,
                cause,
            );
        }
    }

    pub fn is_dirty(&self, node: NodeIndex) -> bool {
        let wd = self.0.borrow();
        wd.dependencies[node].is_dirty()
    }

    pub fn unmark(&self, node: NodeIndex) {
        self.0.borrow_mut().dependencies[node].unmark();
    }

    pub fn reset_dirty_state(&self, node: NodeIndex) -> DirtyFlag {
        replace(
            &mut self.0.borrow_mut().dependencies[node],
            DirtyFlag::clean(),
        )
    }

    pub fn create_node(&self) -> NodeIndex {
        self.0
            .borrow_mut()
            .dependencies
            .add_node(DirtyFlag::dirty())
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
    fn node(&self) -> NodeIndex;
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
    fn node(&self) -> NodeIndex;
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

    fn node(&self) -> NodeIndex {
        self.0.borrow().node()
    }
}

impl<'a, T: Data> PartialEq for StreamSignal<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
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

    pub fn filter_map<O: Data + 'a, F: Fn(T) -> Option<O> + 'a>(
        &self,
        fmap: F,
    ) -> StreamSignal<'a, O> {
        StreamSignal::create(streams::filter_mapper::FilterMapper::new(
            self.world(),
            self.clone(),
            fmap,
        ))
    }

    pub fn fold<V: Data + 'a, F: Fn(V, T) -> V + 'a>(
        &self,
        folder: F,
        initial: V,
    ) -> ValueSignal<'a, V> {
        ValueSignal::create(Folder::new(self.world(), self.clone(), initial, folder))
    }
}

pub struct ValueSignal<'a, T: Data>(
    Rc<RefCell<dyn ComputationCore<ComputationResult = SingleComputationResult<T>> + 'a>>,
);

impl<'a, T: Data> PartialEq for ValueSignal<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

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

    fn node(&self) -> NodeIndex {
        self.0.borrow().node()
    }
}

impl<'a, T: Data + 'a> ValueSignal<'a, T> {
    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> ValueSignal<'a, R> {
        ValueSignal::create(Mapper::new(self.world(), self.clone(), mapper))
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F, initial: T) -> ValueSignal<'a, T> {
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

    pub fn filter_map<O: Data + 'a, F: Fn(T) -> Option<O> + 'a>(
        &self,
        fmap: F,
        initial: O,
    ) -> ValueSignal<'a, O> {
        ValueSignal::create(values::filter_mapper::FilterMapper::new(
            self.world(),
            self.clone(),
            initial,
            fmap,
        ))
    }
}

impl<'a, T: Data + 'a> ValueSignal<'a, ValueSignal<'a, T>> {
    pub fn flatten(&self) -> ValueSignal<'a, T> {
        self.bind(|v| v)
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

pub fn many<'a, T: Data + 'a>(
    world: &World,
    signals: Vec<StreamSignal<'a, T>>,
) -> StreamSignal<'a, T> {
    StreamSignal::create(Many::new(world.clone(), signals))
}

pub fn iter_as_stream<'a, T: Data + 'a, I: Iterator<Item = T> + 'a>(
    world: &World,
    iter: I,
) -> StreamSignal<'a, T> {
    StreamSignal::create(FromIter::new(world.clone(), iter))
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

pub fn bind2<
    'a,
    I1: Data + 'a,
    I2: Data + 'a,
    O: Data + 'a,
    B: Fn(I1, I2) -> ValueSignal<'a, O> + 'a,
>(
    s1: &ValueSignal<'a, I1>,
    s2: &ValueSignal<'a, I2>,
    binder: B,
) -> ValueSignal<'a, O> {
    ValueSignal::create(Binder2::new(s1.world(), s1.clone(), s2.clone(), binder))
}

pub fn zip_map<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a>(
    s1: &StreamSignal<'a, I1>,
    s2: &StreamSignal<'a, I2>,
    mapper: M,
) -> StreamSignal<'a, O> {
    StreamSignal::create(ZipMapper::new(s1.world(), s1.clone(), s2.clone(), mapper))
}

pub fn combine_map<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a>(
    s1: &StreamSignal<'a, I1>,
    s2: &StreamSignal<'a, I2>,
    mapper: M,
) -> StreamSignal<'a, O> {
    StreamSignal::create(CombineMapper::new(
        s1.world(),
        s1.clone(),
        s2.clone(),
        mapper,
    ))
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
        self.parent = signal.clone();
        self.reader = R::new(signal);
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

pub struct NodeState(World, NodeIndex);

impl NodeState {
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

    pub fn reset_dirty_state(&self) -> DirtyFlag {
        self.0.reset_dirty_state(self.1)
    }

    pub fn mark_dirty(&self, cause: DirtyingCause) {
        self.0.mark_dirty(self.1, cause)
    }
}

impl Drop for NodeState {
    fn drop(&mut self) {
        info!("Dropping signal: {}", self.1);
        self.0.destroy_node(self.1)
    }
}

fn read_once<'a, T: Data + 'a>(signal: &ValueSignal<'a, T>) -> T {
    let reader = signal.create_reader();
    let value = signal.compute(reader);
    signal.destroy_reader(reader);
    value.unwrap_changed()
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
    fn test_bind_complex() {
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
    fn test_bind2() {
        let world = World::new();
        let (set1, v1) = mutable(&world, 1);
        let (set2, v2) = mutable(&world, 2);
        let (set_sw1, sw1) = mutable(&world, false);
        let (set_sw2, sw2) = mutable(&world, false);
        let res = bind2(
            &sw1,
            &sw2,
            move |b1, b2| {
                if b1 && b2 {
                    v1.clone()
                } else {
                    v2.clone()
                }
            },
        );
        assert_eq!(2, read_once(&res));
        set2(4);
        assert_eq!(4, read_once(&res));
        set_sw1(true);
        assert_eq!(4, read_once(&res));
        set_sw2(true);
        assert_eq!(1, read_once(&res));
        set2(5);
        assert_eq!(1, read_once(&res));
        set1(6);
        assert_eq!(6, read_once(&res));
        set_sw1(false);
        assert_eq!(5, read_once(&res));
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

    fn collect_current<T: Data>(coll: &Collector<T>) -> Vec<T> {
        coll.iter().map(|t| t.cheap_clone()).collect()
    }

    fn collect_all<T: Data>(coll: &mut Collector<T>) -> Vec<T> {
        coll.clear();
        coll.update();
        collect_current(&coll)
    }

    #[test]
    fn test_collect() {
        let world = World::new();
        let (send, s1) = portal(&world);
        let mut collector = s1.collect();
        assert_eq!(collect_current(&collector), Vec::new());
        send(1);
        assert_eq!(collect_current(&collector), Vec::new());
        collector.update();
        assert_eq!(collect_current(&collector), vec![1]);
        collector.clear();
        send(2);
        send(3);
        send(4);
        collector.update();
        assert_eq!(collect_current(&collector), vec![2, 3, 4]);
    }

    #[test]
    fn test_stream_map() {
        let world = World::new();
        let (send, s1) = portal(&world);
        let c = s1.map(|v| v + 1).last(0);
        assert_eq!(read_once(&c), 0);
        send(1);
        assert_eq!(read_once(&c), 2);
        send(2);
        send(3);
        send(4);
        assert_eq!(read_once(&c), 5);
    }

    #[test]
    fn test_stream_filter() {
        let world = World::new();
        let (send, s1) = portal::<i32>(&world);
        let mut filtered = s1.filter(|v| v % 2 == 0).collect();
        assert_eq!(Vec::<i32>::new(), collect_current(&filtered));
        send(1);
        filtered.update();
        assert_eq!(Vec::<i32>::new(), collect_current(&filtered));
        send(0);
        filtered.update();
        assert_eq!(vec![0], collect_current(&filtered));
        send(1);
        send(2);
        send(3);
        send(5);
        send(6);
        filtered.update();
        assert_eq!(vec![0, 2, 6], collect_current(&filtered));
    }

    #[test]
    fn test_stream_changed() {
        let world = World::new();
        let (set, v1) = mutable(&world, 0);
        let mut changes: Collector<i32> = v1.changed().collect();
        changes.update();
        assert_eq!(vec![0], collect_current(&changes));
        set(1);
        changes.clear();
        changes.update();
        assert_eq!(vec![1], collect_current(&changes));
        set(2);
        set(3);
        changes.update();
        assert_eq!(vec![1, 3], collect_current(&changes));
    }

    #[test]
    fn test_stream_cached() {
        let world = World::new();
        let (send, raw) = portal(&world);
        let mut cached: Collector<i32> = raw.cached().collect();
        cached.update();
        assert_eq!(Vec::<i32>::new(), collect_current(&cached));
        send(1);
        send(1);
        cached.update();
        assert_eq!(vec![1], collect_current(&cached));
        send(2);
        send(3);
        send(4);
        send(3);
        cached.update();
        assert_eq!(vec![1, 2, 3, 4, 3], collect_current(&cached));
    }

    #[test]
    fn test_multiple_streams() {
        let world = World::new();
        let (send, s1) = portal(&world);
        let m1 = s1.map(|i| i + 1).last(0);
        let m2 = s1.map(|i| i + 2).last(0);
        assert_eq!(0, read_once(&m1));
        assert_eq!(0, read_once(&m2));
        send(1);
        assert_eq!(2, read_once(&m1));
        assert_eq!(3, read_once(&m2));
        send(3);
        assert_eq!(4, read_once(&m1));
        assert_eq!(5, read_once(&m2));
    }

    #[test]
    fn test_stream_zip_map() {
        let world = World::new();
        let (push1, s1) = portal(&world);
        let (push2, s2) = portal(&world);
        let r = zip_map(&s1, &s2, |i1, i2| (i1, i2));
        let mut coll: Collector<(i32, i32)> = r.collect();
        assert_eq!(Vec::<(i32, i32)>::new(), collect_all(&mut coll));
        push1(1);
        push1(2);
        assert_eq!(Vec::<(i32, i32)>::new(), collect_all(&mut coll));
        push2(1);
        assert_eq!(vec![(1, 1)], collect_all(&mut coll));
        push2(3);
        assert_eq!(vec![(2, 3)], collect_all(&mut coll));
        push2(4);
        assert_eq!(Vec::<(i32, i32)>::new(), collect_all(&mut coll));
        push1(0);
        push2(0);
        assert_eq!(vec![(0, 4)], collect_all(&mut coll));
    }

    #[test]
    fn test_stream_combine_map() {
        let world = World::new();
        let (push1, s1) = portal(&world);
        let (push2, s2) = portal(&world);
        let c = combine_map(&s1, &s2, |v1, v2| (v1, v2));
        let mut coll = c.collect();
        push1(1);
        assert_eq!(Vec::<(i32, i32)>::new(), collect_all(&mut coll));
        push2(2);
        assert_eq!(vec![(1, 2)], collect_all(&mut coll));
        push1(3);
        assert_eq!(vec![(3, 2)], collect_all(&mut coll));
        push1(4);
        assert_eq!(vec![(4, 2)], collect_all(&mut coll));
        push1(5);
        push2(5);
        assert_eq!(vec![(5, 5)], collect_all(&mut coll));
    }

    #[test]
    fn test_filter_map() {
        let world = World::new();
        let (set, v) = mutable(&world, 0);
        let fmapped = v.filter_map(|i| if i % 2 == 0 { Some(i) } else { None }, 0);
        assert_eq!(0, read_once(&fmapped));
        set(1);
        assert_eq!(0, read_once(&fmapped));
        set(2);
        assert_eq!(2, read_once(&fmapped));
        set(4);
        assert_eq!(4, read_once(&fmapped));
        set(5);
        assert_eq!(4, read_once(&fmapped));
    }

    #[test]
    fn test_filter_map_stream() {
        let world = World::new();
        let (send, s) = portal(&world);
        let mut r = s
            .filter_map(|i| if i % 2 == 0 { Some(i) } else { None })
            .collect();
        send(0);
        assert_eq!(vec![0], collect_all(&mut r));
        send(1);
        send(2);
        send(3);
        send(4);
        send(5);
        assert_eq!(vec![2, 4], collect_all(&mut r));
    }

    #[test]
    fn test_fold() {
        let world = World::new();
        let (send, s) = portal(&world);
        let v = s.fold(|i, s| s + i, 0);
        assert_eq!(0, read_once(&v));
        send(1);
        assert_eq!(1, read_once(&v));
        send(4);
        send(5);
        send(2);
        assert_eq!(12, read_once(&v));
    }

    #[test]
    fn test_many() {
        let world = World::new();
        let (send1, s1) = portal(&world);
        let (send2, s2) = portal(&world);
        let mut c = many(&world, vec![s1, s2]).collect();
        send1(1);
        send2(1);
        send1(2);
        assert_eq!(vec![1, 1, 2], collect_all(&mut c));
        send1(3);
        send2(4);
        assert_eq!(vec![3, 4], collect_all(&mut c));
    }
}
