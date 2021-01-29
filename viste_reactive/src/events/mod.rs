use daggy::stable_dag::StableDag;
use daggy::{NodeIndex, Walker};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ops::IndexMut;
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
}

pub struct Node<'a, T> {
    index: NodeIndex,
    world: World,
    recompute: Box<dyn Fn() -> T + 'a>,
    current_value: RefCell<T>,
}
