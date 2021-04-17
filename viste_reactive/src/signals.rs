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

    fn world(&self) -> &World {
        self.node.world()
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

    fn world(&self) -> &World {
        self.node.world()
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

    fn world(&self) -> &World {
        self.node.world()
    }
}

impl<I: Data, O: Data, B: Fn(&I) -> Signal<O>> Drop for Binder<I, O, B> {
    fn drop(&mut self) {
        self.current_signal.remove_dependency(self.node.node())
    }
}

struct Mapper2<I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O> {
    source1: ParentSignal<I1>,
    source2: ParentSignal<I2>,
    current_value: O,
    mapper: M,
    node: OwnNode,
}

impl<I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O> Mapper2<I1, I2, O, M> {
    pub fn new(world: World, source1: Signal<I1>, source2: Signal<I2>, mapper: M) -> Self {
        let node = OwnNode::new(world);
        let initial_value = mapper(&source1.compute(), &source2.compute());
        let source1 = ParentSignal::new(source1, node.node());
        let source2 = ParentSignal::new(source2, node.node());
        Self {
            mapper,
            node,
            current_value: initial_value,
            source1,
            source2,
        }
    }
}

impl<I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O> RSignal<O> for Mapper2<I1, I2, O, M> {
    fn compute(&mut self) -> O {
        if self.node.is_dirty() {
            self.node.clean();
            self.current_value = (self.mapper)(&self.source1.compute(), &self.source2.compute())
        }
        self.current_value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.remove_dependency(child)
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}

struct Mutable<T: Data> {
    current_value: T,
    node: OwnNode,
}

impl<T: Data> Mutable<T> {
    pub fn new(world: World, initial: T) -> Self {
        let node = OwnNode::new(world);
        Self {
            current_value: initial,
            node,
        }
    }

    pub fn set(&mut self, value: T) {
        self.current_value = value;
        self.node.mark_dirty();
    }
}

impl<T: Data> RSignal<T> for Mutable<T> {
    fn compute(&mut self) -> T {
        self.current_value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}

struct Constant<T: Data> {
    node: OwnNode,
    value: T,
}

impl<T: Data> Constant<T> {
    pub fn new(world: World, value: T) -> Self {
        Self {
            node: OwnNode::new(world),
            value,
        }
    }
}

impl<T: Data> RSignal<T> for Constant<T> {
    fn compute(&mut self) -> T {
        self.value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) {}

    fn remove_dependency(&mut self, child: NodeIndex) {}

    fn world(&self) -> &World {
        self.node.world()
    }
}
