use crate::graph::{Graph, NodeIndex, SearchContinuation};
use crate::signals::binder::Binder;
use crate::signals::constant::Constant;
use crate::signals::filter::Filter;
use crate::signals::mapper::{Mapper, Mapper2};
use crate::signals::mutable::Mutable;
use crate::Data;
use log::info;
use slab::Slab;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::rc::Rc;

mod binder;
mod constant;
mod filter;
mod mapper;
mod mutable;

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

    pub fn mutable<'a, T: Data + 'a>(&self, initial: T) -> (impl Fn(T), Signal<'a, T>) {
        let m = Mutable::new(self.clone(), initial);
        let signal = Rc::new(RefCell::new(m));
        let s = signal.clone();
        let mutator = move |v| s.borrow_mut().set(v);
        (mutator, Signal(signal))
    }

    pub fn constant<'a, T: Data + 'a>(&self, value: T) -> Signal<'a, T> {
        Signal::create(Constant::new(self.clone(), value))
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

pub trait ComputationCore<T: Data> {
    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<T>;
    fn create_reader(&mut self) -> ReaderToken;
    fn destroy_reader(&mut self, reader: ReaderToken);
    fn add_dependency(&mut self, child: NodeIndex);
    fn remove_dependency(&mut self, child: NodeIndex);
    fn is_dirty(&self) -> bool;
    fn world(&self) -> &World;
}

pub struct Signal<'a, T: Data>(Rc<RefCell<dyn ComputationCore<T> + 'a>>);

impl<'a, T: Data + 'a> Signal<'a, T> {
    pub fn create<S: ComputationCore<T> + 'a>(r: S) -> Self {
        Self(Rc::new(RefCell::new(r)))
    }

    pub fn world(&self) -> World {
        self.0.borrow().world().clone()
    }

    pub fn compute(&self, reader: ReaderToken) -> SingleComputationResult<T> {
        self.0.borrow_mut().compute(reader)
    }

    pub fn add_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().add_dependency(child)
    }

    pub fn remove_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().remove_dependency(child)
    }

    pub fn create_reader(&self) -> ReaderToken {
        self.0.borrow_mut().create_reader()
    }

    pub fn destroy_reader(&self, reader: ReaderToken) {
        self.0.borrow_mut().destroy_reader(reader)
    }

    pub fn is_dirty(&self) -> bool {
        self.0.borrow().is_dirty()
    }

    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> Signal<'a, R> {
        Signal::create(Mapper::new(self.world(), self.clone(), mapper))
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(self, filter: F, initial: T) -> Signal<'a, T> {
        Signal::create(Filter::new(self.world(), self.clone(), initial, filter))
    }

    pub fn bind<O: Data + 'a, B: Fn(T) -> Signal<'a, O> + 'a>(&self, binder: B) -> Signal<'a, O> {
        Signal::create(Binder::new(self.world(), self.clone(), binder))
    }
}

impl<'a, T: Data + 'a> Debug for Signal<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dirty = self.is_dirty();
        let value = read_once(self);
        write!(f, "Signal {{ dirty: {}, value: {:?} }}", dirty, value)
    }
}

pub fn map2<'a, T1: Data + 'a, T2: Data + 'a, O: Data + 'a, M: Fn(T1, T2) -> O + 'a>(
    s1: &Signal<'a, T1>,
    s2: &Signal<'a, T2>,
    mapper: M,
) -> Signal<'a, O> {
    Signal::create(Mapper2::new(s1.world(), s1.clone(), s2.clone(), mapper))
}

impl<'a, T: Data> Clone for Signal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct ParentSignal<'a, T: Data + 'a, S, R: Reader<'a, T, S>> {
    parent: Signal<'a, T>,
    own_index: NodeIndex,
    reader: R,
    _data: PhantomData<S>,
}

impl<'a, T: Data + 'a, S, R: Reader<'a, T, S>> ParentSignal<'a, T, S, R> {
    pub fn new(signal: Signal<'a, T>, own_index: NodeIndex) -> Self {
        signal.add_dependency(own_index);
        let reader = R::new(signal.clone());
        Self {
            parent: signal,
            own_index,
            reader,
            _data: PhantomData,
        }
    }

    pub fn set_parent(&mut self, signal: Signal<'a, T>) {
        self.parent.remove_dependency(self.own_index);
        signal.add_dependency(self.own_index);
        self.parent = signal;
    }

    pub fn compute(&mut self) -> S {
        self.reader.read()
    }
}

impl<'a, T: Data + 'a, S, R: Reader<'a, T, S>> Drop for ParentSignal<'a, T, S, R> {
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

fn read_once<'a, T: Data + 'a>(signal: &Signal<'a, T>) -> T {
    let reader = signal.create_reader();
    let value = signal.compute(reader);
    signal.destroy_reader(reader);
    value.unwrap_changed()
}

pub trait Reader<'a, T: Data + 'a, R> {
    fn new(signal: Signal<'a, T>) -> Self;
    fn read(&mut self) -> R;
}

pub struct ChangeReader<'a, T: Data + 'a>(Signal<'a, T>, ReaderToken);

impl<'a, T: Data + 'a> Reader<'a, T, SingleComputationResult<T>> for ChangeReader<'a, T> {
    fn new(signal: Signal<'a, T>) -> Self {
        let reader = signal.create_reader();
        Self(signal, reader)
    }

    fn read(&mut self) -> SingleComputationResult<T> {
        self.0.compute(self.1)
    }
}

impl<'a, T: Data + 'a> Drop for ChangeReader<'a, T> {
    fn drop(&mut self) {
        self.0.destroy_reader(self.1)
    }
}

pub struct CachedReader<'a, T: Data + 'a> {
    signal: Signal<'a, T>,
    token: ReaderToken,
    cache: T,
}

impl<'a, T: Data + 'a> Reader<'a, T, (bool, T)> for CachedReader<'a, T> {
    fn new(signal: Signal<'a, T>) -> Self {
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

#[cfg(test)]
mod tests {
    use crate::signals::*;

    #[test]
    fn test_map() {
        let world = World::new();
        let (set, v) = world.mutable(3);
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
        let (set1, v1) = world.mutable(0);
        let (set2, v2) = world.mutable(0);
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
        let (set1, v1) = world.mutable(0);
        let v2 = world.constant(1);
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
        let (set, v) = world.mutable(0);
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
        let c = world.constant(2);
        let (set, v) = world.mutable(0);
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
        let (set1, v1) = world.mutable(1);
        let (set2, v2) = world.mutable(2);
        let (switch, switcher) = world.mutable(false);
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
}
