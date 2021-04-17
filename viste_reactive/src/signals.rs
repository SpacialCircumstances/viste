use crate::graph::{Graph, NodeIndex, SearchContinuation};
use crate::Data;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

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

pub trait RSignal<T: Data> {
    fn compute(&mut self) -> T;
    fn add_dependency(&mut self, child: NodeIndex);
    fn remove_dependency(&mut self, child: NodeIndex);
}

pub struct Signal<T: Data>(Rc<RefCell<dyn RSignal<T>>>);

impl<T: Data> Signal<T> {
    pub fn compute(&self) -> T {
        self.0.borrow_mut().compute()
    }

    pub fn add_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().add_dependency(child)
    }

    pub fn remove_dependency(&self, child: NodeIndex) {
        self.0.borrow_mut().remove_dependency(child)
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
}

impl Drop for OwnNode {
    fn drop(&mut self) {
        self.0.destroy_node(self.1)
    }
}

struct Mapper<I: Data, O: Data, M: Fn(I) -> O> {
    source: ParentSignal<I>,
    current_value: O,
    mapper: M,
    node: OwnNode,
}

impl<I: Data, O: Data, M: Fn(I) -> O> Mapper<I, O, M> {
    pub fn new(world: World, source: Signal<I>, mapper: M) -> Self {
        let current_value = mapper(source.compute());
        let node = OwnNode::new(world);
        Mapper {
            source: ParentSignal::new(source, node.node()),
            mapper,
            current_value,
            node,
        }
    }
}

impl<I: Data, O: Data, M: Fn(I) -> O> RSignal<O> for Mapper<I, O, M> {
    fn compute(&mut self) -> O {
        if self.node.is_dirty() {
            self.current_value = (self.mapper)(self.source.compute());
            self.node.clean();
        }
        self.current_value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.remove_dependency(child)
    }
}

struct Filter<T: Data, F: Fn(&T) -> bool> {
    source: ParentSignal<T>,
    current_value: T,
    filter: F,
    node: OwnNode,
}

impl<T: Data, F: Fn(&T) -> bool> Filter<T, F> {
    pub fn new(world: World, parent: Signal<T>, initial: T, filter: F) -> Self {
        let node = OwnNode::new(world);
        let source = ParentSignal::new(parent, node.node());
        Self {
            source,
            filter,
            node,
            current_value: initial,
        }
    }
}

impl<T: Data, F: Fn(&T) -> bool> RSignal<T> for Filter<T, F> {
    fn compute(&mut self) -> T {
        if self.node.is_dirty() {
            self.node.clean();
            let new_source = self.source.compute();
            if (self.filter)(&new_source) {
                self.current_value = new_source;
            }
        }
        self.current_value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.remove_dependency(child)
    }
}

struct Binder<I: Data, O: Data, B: Fn(&I) -> Signal<O>> {
    binder: B,
    current_signal: Signal<O>,
    node: OwnNode,
    parent: ParentSignal<I>,
}

impl<I: Data, O: Data, B: Fn(&I) -> Signal<O>> Binder<I, O, B> {
    pub fn new(world: World, parent: Signal<I>, binder: B) -> Self {
        let initial_signal = binder(&parent.compute());
        let node = OwnNode::new(world);
        initial_signal.add_dependency(node.node());
        let parent = ParentSignal::new(parent, node.node());
        Binder {
            binder,
            node,
            parent,
            current_signal: initial_signal,
        }
    }
}

impl<I: Data, O: Data, B: Fn(&I) -> Signal<O>> RSignal<O> for Binder<I, O, B> {
    fn compute(&mut self) -> O {
        if self.node.is_dirty() {
            self.node.clean();
            let new_source = self.parent.compute();
            self.current_signal.remove_dependency(self.node.node());
            self.current_signal = (self.binder)(&new_source);
            self.current_signal.add_dependency(self.node.node());
        }
        self.current_signal.compute()
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.remove_dependency(child)
    }
}

impl<I: Data, O: Data, B: Fn(&I) -> Signal<O>> Drop for Binder<I, O, B> {
    fn drop(&mut self) {
        self.current_signal.remove_dependency(self.node.node())
    }
}
