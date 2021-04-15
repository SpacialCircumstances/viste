use crate::old::graph::{Graph, NodeIndex, SearchContinuation};
use crate::Data;
use std::cell::{Ref, RefCell};
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

struct Mapper<I: Data, O: Data, M: Fn(I) -> O> {
    source: Signal<I>,
    current_value: O,
    mapper: M,
    world: World,
    node: NodeIndex,
}

impl<I: Data, O: Data, M: Fn(I) -> O> Mapper<I, O, M> {
    pub fn new(world: World, source: Signal<I>, mapper: M) -> Self {
        let current_value = mapper(source.compute());
        let node = world.create_node();
        Mapper {
            source,
            world,
            mapper,
            current_value,
            node,
        }
    }
}

impl<I: Data, O: Data, M: Fn(I) -> O> RSignal<O> for Mapper<I, O, M> {
    fn compute(&mut self) -> O {
        if self.world.is_dirty(self.node) {
            self.current_value = (self.mapper)(self.source.compute())
        }
        self.current_value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.world.add_dependency(self.node, child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.world.remove_dependency(self.node, child)
    }
}

impl<I: Data, O: Data, M: Fn(I) -> O> Drop for Mapper<I, O, M> {
    fn drop(&mut self) {
        self.world.destroy_node(self.node)
    }
}
