use daggy::stable_dag::StableDag;
use daggy::{NodeIndex, Walker};
use std::cell::{Ref, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

struct WorldData {
    dependencies: StableDag<bool, ()>,
}

pub struct World(Rc<RefCell<WorldData>>);

impl World {
    pub fn new() -> Self {
        World(Rc::new(RefCell::new(WorldData {
            dependencies: StableDag::new(),
        })))
    }

    pub fn mark_dirty(&self, node: NodeIndex) {
        let mut wd = self.0.borrow_mut();
        let old_dirty = wd.dependencies[node];
        if !old_dirty {
            let mut roots_to_mark = VecDeque::new();
            roots_to_mark.push_back(node);

            while let Some(root) = roots_to_mark.pop_front() {
                wd.dependencies[root] = true;
                wd.dependencies
                    .children(root)
                    .iter(&wd.dependencies)
                    .filter(|(_edge, child)| !wd.dependencies[*child])
                    .for_each(|(_edge, r)| roots_to_mark.push_back(r));
            }
        }
    }

    pub fn is_dirty(&self, node: NodeIndex) -> bool {
        let wd = self.0.borrow();
        wd.dependencies[node]
    }

    pub fn unmark(&self, node: NodeIndex) {
        self.0.borrow_mut().dependencies[node] = false;
    }

    pub fn destroy_node(&self, node: NodeIndex) {
        self.0.borrow_mut().dependencies.remove_node(node);
    }

    pub fn create_node<'a, T, F: Fn(NodeIndex, &mut T) -> ComputationResult + 'a>(
        &self,
        change: F,
        initial: T,
    ) -> Node<'a, T> {
        let index = self.0.borrow_mut().dependencies.add_node(false);

        let data = NodeData {
            index,
            world: self.clone(),
            change: Box::new(change),
            current_value: RefCell::new(initial),
        };
        Node(Rc::new(data))
    }

    pub fn add_dependency(&self, parent: NodeIndex, child: NodeIndex) {
        self.0
            .borrow_mut()
            .dependencies
            .add_edge(parent, child, ())
            .expect("Dependency cycle");
    }

    pub fn remove_dependency(&self, parent: NodeIndex, child: NodeIndex) {
        let mut wd = self.0.borrow_mut();
        let edge = wd
            .dependencies
            .find_edge(parent, child)
            .expect("Tried to remove dependency that does not exist");
        wd.dependencies.remove_edge(edge);
    }

    pub fn constant<'a, T>(&self, value: T) -> Node<'a, T> {
        self.create_node(move |_, _| ComputationResult::Unchanged, value)
    }

    pub fn mutable<'a, T: Clone + 'a>(&self, initial: T) -> (Mutable<T>, Node<'a, T>) {
        //TODO: Make nodes that do not cache their values possible
        let world = self.clone();
        let store = Rc::new(RefCell::new(initial.clone()));
        let value_store = store.clone();
        let node = self.create_node(
            move |_idx, t| {
                *t = store.borrow().clone();
                ComputationResult::Changed
            },
            initial,
        );
        let mutable = Mutable {
            world,
            value_store,
            index: node.0.index,
        };
        (mutable, node)
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

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ComputationResult {
    Changed,
    Unchanged,
}

struct NodeData<'a, T> {
    index: NodeIndex,
    world: World,
    change: Box<dyn Fn(NodeIndex, &mut T) -> ComputationResult + 'a>,
    current_value: RefCell<T>,
}

pub struct Node<'a, T>(Rc<NodeData<'a, T>>);

impl<'a, T> Clone for Node<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T> Drop for Node<'a, T> {
    fn drop(&mut self) {
        self.0.world.destroy_node(self.0.index);
    }
}

impl<'a, T: 'a> Node<'a, T> {
    pub fn data(&self) -> (Ref<T>, ComputationResult) {
        let mut res = ComputationResult::Unchanged;
        if self.0.world.is_dirty(self.0.index) {
            let mut value = self.0.current_value.borrow_mut();
            res = (self.0.change)(self.0.index, &mut *value);
            self.0.world.unmark(self.0.index);
        }
        (self.0.current_value.borrow(), res)
    }

    pub fn with<O, F: FnOnce(&T) -> O>(&self, f: F) -> O {
        let val = self.0.current_value.borrow();
        f(&*val)
    }

    pub fn is_dirty(&self) -> bool {
        self.0.world.is_dirty(self.0.index)
    }

    pub fn add_depending(&self, child: NodeIndex) {
        self.0.world.add_dependency(self.0.index, child);
    }

    pub fn remove_depending(&self, child: NodeIndex) {
        self.0.world.remove_dependency(self.0.index, child);
    }

    pub fn map<Z, M: Fn(&T) -> Z + 'a>(&self, mapper: M) -> Node<'a, Z> {
        let initial = mapper(&*self.data().0);
        let this = self.clone();
        let node = self.0.world.create_node(
            move |_idx, t| {
                //We cannot rely on the mapper functions purity, so we can't pass the change-tracking.
                *t = mapper(&*this.data().0);
                ComputationResult::Changed
            },
            initial,
        );
        self.add_depending(node.0.index);
        node
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F, initial: T) -> Node<'a, T>
    where
        T: Clone,
    {
        let this = self.clone();
        let current_data = self.data().0;
        let initial = if filter(&*current_data) {
            current_data.clone()
        } else {
            initial
        };
        let node = self.0.world.create_node(
            move |_idx, t| {
                let (v, changed) = this.data();
                if changed == ComputationResult::Changed && filter(&*v) {
                    *t = v.clone();
                    ComputationResult::Changed
                } else {
                    ComputationResult::Unchanged
                }
            },
            initial,
        );
        self.add_depending(node.0.index);
        node
    }

    pub fn bind<Z: Clone + 'a, B: Fn(&T) -> Node<'a, Z> + 'a>(&self, binder: B) -> Node<'a, Z> {
        let this = self.clone();
        let curr_data = self.data().0;
        let current_node = binder(&*curr_data);
        let cn_idx = current_node.0.index;
        let initial = current_node.data().0.clone();
        let node_store = RefCell::new(current_node);
        let node = self.0.world.create_node(
            move |idx, t| {
                let (v, changed) = this.data();
                if changed == ComputationResult::Changed {
                    let new_node = binder(&*v);
                    new_node.add_depending(idx);
                    node_store.replace(new_node).remove_depending(idx);
                }

                let store = node_store.borrow();
                let (v, changed) = store.data();
                if changed == ComputationResult::Changed {
                    *t = v.clone();
                }
                changed
            },
            initial,
        );
        self.add_depending(node.0.index);
        self.0.world.add_dependency(cn_idx, node.0.index);
        node
    }
}

pub struct Mutable<T> {
    world: World,
    index: NodeIndex,
    value_store: Rc<RefCell<T>>,
}

impl<T> Mutable<T> {
    pub fn set(&mut self, value: T) {
        self.value_store.replace(value);
        self.world.mark_dirty(self.index);
    }
}
