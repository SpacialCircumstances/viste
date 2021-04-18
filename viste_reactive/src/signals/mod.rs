use crate::graph::{Graph, NodeIndex, SearchContinuation};
use crate::signals::binder::Binder;
use crate::signals::constant::Constant;
use crate::signals::filter::Filter;
use crate::signals::mapper::{Mapper, Mapper2};
use crate::signals::mutable::Mutable;
use crate::Data;
use slab::Slab;
use std::cell::RefCell;
use std::ops::Deref;
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

pub trait SignalCore<T: Data> {
    fn compute(&mut self, reader: ReaderToken) -> T;
    fn create_reader(&mut self) -> ReaderToken;
    fn destroy_reader(&mut self, reader: ReaderToken);
    fn add_dependency(&mut self, child: NodeIndex);
    fn remove_dependency(&mut self, child: NodeIndex);
    fn world(&self) -> &World;
}

pub struct Signal<'a, T: Data>(Rc<RefCell<dyn SignalCore<T> + 'a>>);

impl<'a, T: Data + 'a> Signal<'a, T> {
    pub fn create<S: SignalCore<T> + 'a>(r: S) -> Self {
        Self(Rc::new(RefCell::new(r)))
    }

    pub fn world(&self) -> World {
        self.0.borrow().world().clone()
    }

    pub fn compute(&self, reader: ReaderToken) -> T {
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

    pub fn map<R: Data + 'a, M: Fn(T) -> R + 'a>(&self, mapper: M) -> Signal<'a, R> {
        Signal::create(Mapper::new(self.world(), self.clone(), mapper))
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(self, filter: F, initial: T) -> Signal<'a, T> {
        Signal::create(Filter::new(self.world(), self.clone(), initial, filter))
    }

    pub fn bind<O: Data + 'a, B: Fn(&T) -> Signal<'a, O> + 'a>(&self, binder: B) -> Signal<'a, O> {
        Signal::create(Binder::new(self.world(), self.clone(), binder))
    }
}

pub fn map2<'a, T1: Data + 'a, T2: Data + 'a, O: Data + 'a, M: Fn(&T1, &T2) -> O + 'a>(
    s1: Signal<'a, T1>,
    s2: Signal<'a, T2>,
    mapper: M,
) -> Signal<'a, O> {
    Signal::create(Mapper2::new(s1.world(), s1, s2, mapper))
}

impl<'a, T: Data> Clone for Signal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct ParentSignal<'a, T: Data + 'a> {
    parent: Signal<'a, T>,
    own_index: NodeIndex,
    reader: ReaderToken,
}

impl<'a, T: Data + 'a> ParentSignal<'a, T> {
    pub fn new(signal: Signal<'a, T>, own_index: NodeIndex) -> Self {
        signal.add_dependency(own_index);
        let reader = signal.create_reader();
        Self {
            parent: signal,
            own_index,
            reader,
        }
    }

    pub fn set_parent(&mut self, signal: Signal<'a, T>) {
        self.parent.remove_dependency(self.own_index);
        signal.add_dependency(self.own_index);
        self.parent = signal;
    }

    pub fn compute(&self) -> T {
        self.parent.compute(self.reader)
    }
}

impl<'a, T: Data + 'a> Drop for ParentSignal<'a, T> {
    fn drop(&mut self) {
        self.parent.remove_dependency(self.own_index);
        self.parent.destroy_reader(self.reader)
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
        self.0.destroy_node(self.1)
    }
}

pub struct SingleValueStore<T: Data> {
    value: T,
    current_reader_index: usize,
}

impl<T: Data> SingleValueStore<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            current_reader_index: 0,
        }
    }

    pub fn create_reader(&mut self) -> ReaderToken {
        let reader = ReaderToken(self.current_reader_index);
        self.current_reader_index += 1;
        reader
    }

    pub fn destroy_reader(&mut self, reader: ReaderToken) {}

    pub fn set_value(&mut self, value: T) {
        self.value = value;
    }

    pub fn read(&mut self, reader: ReaderToken) -> T {
        self.value.cheap_clone()
    }
}
