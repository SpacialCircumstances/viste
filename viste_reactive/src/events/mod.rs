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

    pub fn create_node<'a, T, F: Fn(&mut T) -> ComputationResult + 'a>(
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
    change: Box<dyn Fn(&mut T) -> ComputationResult + 'a>,
    current_value: RefCell<T>,
}

pub struct Node<'a, T>(Rc<NodeData<'a, T>>);

impl<'a, T> Clone for Node<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T: 'a> Node<'a, T> {
    pub fn data(&self) -> (Ref<T>, ComputationResult) {
        let mut res = ComputationResult::Unchanged;
        if self.0.world.is_dirty(self.0.index) {
            let mut value = self.0.current_value.borrow_mut();
            res = (self.0.change)(&mut *value);
            self.0.world.unmark(self.0.index);
        }
        (self.0.current_value.borrow(), res)
    }

    pub fn with<O, F: FnOnce(&T) -> O>(&self, f: F) -> O {
        let val = self.0.current_value.borrow();
        f(&*val)
    }

    pub fn map<Z, M: Fn(&T) -> Z + 'a>(&self, mapper: M) -> Node<'a, Z> {
        let initial = mapper(&*self.data().0);
        let this = self.clone();
        self.0.world.create_node(
            move |t| {
                //We cannot rely on the mapper functions purity, so we can't pass the change-tracking.
                *t = mapper(&*this.data().0);
                ComputationResult::Changed
            },
            initial,
        )
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
        self.0.world.create_node(
            move |t| {
                let (v, changed) = this.data();
                if changed == ComputationResult::Changed && filter(&*v) {
                    *t = v.clone();
                    ComputationResult::Changed
                } else {
                    ComputationResult::Unchanged
                }
            },
            initial,
        )
    }
}
