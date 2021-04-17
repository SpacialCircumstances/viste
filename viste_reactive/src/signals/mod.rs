use crate::graph::{Graph, NodeIndex, SearchContinuation};
use crate::signals::binder::Binder;
use crate::signals::constant::Constant;
use crate::signals::filter::Filter;
use crate::signals::mapper::{Mapper, Mapper2};
use crate::signals::mutable::Mutable;
use crate::Data;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

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

    pub fn mutable<T: Data + 'static>(&self, initial: T) -> (impl Fn(T), Signal<T>) {
        let m = Mutable::new(self.clone(), initial);
        let signal = Rc::new(RefCell::new(m));
        let s = signal.clone();
        let mutator = move |v| s.borrow_mut().set(v);
        (mutator, Signal(signal))
    }

    pub fn constant<T: Data + 'static>(&self, value: T) -> Signal<T> {
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

pub trait RSignal<T: Data> {
    fn compute(&mut self) -> T;
    fn add_dependency(&mut self, child: NodeIndex);
    fn remove_dependency(&mut self, child: NodeIndex);
    fn world(&self) -> &World;
}

pub struct Signal<T: Data>(Rc<RefCell<dyn RSignal<T>>>);

impl<T: Data> Signal<T> {
    pub fn create<S: RSignal<T> + 'static>(r: S) -> Self {
        Self(Rc::new(RefCell::new(r)))
    }

    pub fn world(&self) -> World {
        self.0.borrow().world().clone()
    }

    pub fn compute(&self) -> T {
        self.0.borrow_mut().compute()
    }

    pub fn add_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().add_dependency(child)
    }

    pub fn remove_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().remove_dependency(child)
    }

    pub fn map<R: Data, M: Fn(T) -> R + 'static>(&self, mapper: M) -> Signal<R> {
        Signal::create(Mapper::new(self.world(), self.clone(), mapper))
    }

    pub fn filter<F: Fn(&T) -> bool + 'static>(self, filter: F, initial: T) -> Signal<T> {
        Signal::create(Filter::new(self.world(), self.clone(), initial, filter))
    }

    pub fn bind<O: Data, B: Fn(&T) -> Signal<O> + 'static>(&self, binder: B) -> Signal<O> {
        Signal::create(Binder::new(self.world(), self.clone(), binder))
    }
}

pub fn map2<T1: Data, T2: Data, O: Data, M: Fn(&T1, &T2) -> O + 'static>(
    s1: Signal<T1>,
    s2: Signal<T2>,
    mapper: M,
) -> Signal<O> {
    Signal::create(Mapper2::new(s1.world(), s1, s2, mapper))
}

impl<T: Data> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct WeakSignal<T: Data>(Weak<RefCell<dyn RSignal<T>>>);

impl<T: Data> WeakSignal<T> {
    pub fn upgrade(&self) -> Option<Signal<T>> {
        self.0.upgrade().map(|s| Signal(s))
    }
}

pub struct ParentSignal<T: Data> {
    parent: Signal<T>,
    own_index: NodeIndex,
}

impl<T: Data> ParentSignal<T> {
    pub fn new(signal: Signal<T>, own_index: NodeIndex) -> Self {
        signal.add_dependency(own_index);
        Self {
            parent: signal,
            own_index,
        }
    }

    pub fn set_parent(&mut self, signal: Signal<T>) {
        self.parent.remove_dependency(self.own_index);
        signal.add_dependency(self.own_index);
        self.parent = signal;
    }
}

impl<T: Data> Deref for ParentSignal<T> {
    type Target = Signal<T>;

    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}

impl<T: Data> Drop for ParentSignal<T> {
    fn drop(&mut self) {
        self.parent.remove_dependency(self.own_index)
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
