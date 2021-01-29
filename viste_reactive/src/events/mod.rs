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

    pub fn create_node<'a, T, F: Fn() -> T + 'a>(&self, recompute: F) -> Node<'a, T> {
        let index = self.0.borrow_mut().dependencies.add_node(false);
        let initial = recompute();

        let data = NodeData {
            index,
            world: self.clone(),
            recompute: Box::new(recompute),
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

struct NodeData<'a, T> {
    index: NodeIndex,
    world: World,
    recompute: Box<dyn Fn() -> T + 'a>,
    current_value: RefCell<T>,
}

pub struct Node<'a, T>(Rc<NodeData<'a, T>>);

impl<'a, T> Node<'a, T> {
    pub fn data(&self) -> Ref<T> {
        if self.0.world.is_dirty(self.0.index) {
            *self.0.current_value.borrow_mut() = (self.0.recompute)();
            self.0.world.unmark(self.0.index);
        }
        self.0.current_value.borrow()
    }

    pub fn with<O, F: FnOnce(&T) -> O>(&self, f: F) -> O {
        let val = self.0.current_value.borrow();
        f(&*val)
    }
}
