use crate::old::graph::{Graph, NodeIndex, SearchContinuation};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

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

    pub fn destroy_node(&self, node: NodeIndex) {
        self.0.borrow_mut().dependencies.remove_node(node);
    }

    pub fn create_node<'a, T, F: Fn(NodeIndex, &mut T) -> ComputationResult + 'a>(
        &self,
        change: F,
        initial: T,
    ) -> Signal<'a, T> {
        let index = self.0.borrow_mut().dependencies.add_node(false);

        let data = NodeData {
            index,
            world: self.clone(),
            change: Box::new(change),
            current_value: RefCell::new(initial),
        };
        Signal(Rc::new(data))
    }

    pub fn add_dependency(&self, parent: NodeIndex, child: NodeIndex) {
        self.0.borrow_mut().dependencies.add_edge(parent, child);
    }

    pub fn remove_dependency(&self, parent: NodeIndex, child: NodeIndex) {
        let mut wd = self.0.borrow_mut();
        wd.dependencies.remove_edge(parent, child);
    }

    pub fn constant<'a, T>(&self, value: T) -> Signal<'a, T> {
        self.create_node(move |_, _| ComputationResult::Unchanged, value)
    }

    pub fn mutable<'a, T: 'a>(&self, initial: T) -> (Mutable<'a, T>, Signal<'a, T>) {
        let world = self.clone();
        let store = Rc::new(RefCell::new(None));
        let value_store = store.clone();
        let node = self.create_node(
            move |_idx, t| match store.replace(None) {
                Some(new_val) => {
                    *t = new_val;
                    ComputationResult::Changed
                }
                None => ComputationResult::Unchanged,
            },
            initial,
        );
        let mutable = Mutable {
            world,
            value_store,
            index: node.0.index,
            _node: node.clone(),
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
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

impl<'a, T> Drop for NodeData<'a, T> {
    fn drop(&mut self) {
        self.world.destroy_node(self.index);
    }
}

pub struct Signal<'a, T>(Rc<NodeData<'a, T>>);

impl<'a, T> Clone for Signal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T: 'a> Signal<'a, T> {
    fn data(&self) -> (Ref<T>, ComputationResult) {
        let mut res = ComputationResult::Unchanged;
        if self.0.world.is_dirty(self.0.index) {
            let mut value = self.0.current_value.borrow_mut();
            res = (self.0.change)(self.0.index, &mut *value);
            self.0.world.unmark(self.0.index);
        }
        (self.0.current_value.borrow(), res)
    }

    pub fn with_data<R, F: FnOnce(&T, ComputationResult) -> R>(&self, then: F) -> R {
        let (data, cr) = self.data();
        then(&*data, cr)
    }

    pub fn if_changed<R, F: FnOnce(&T) -> R>(&self, then: F) -> Option<R> {
        if let (new_data, ComputationResult::Changed) = self.data() {
            Some(then(&*new_data))
        } else {
            None
        }
    }

    pub fn changed(&self) -> Option<T>
    where
        T: Clone,
    {
        if let (new_data, ComputationResult::Changed) = self.data() {
            Some(new_data.clone())
        } else {
            None
        }
    }

    pub fn cloned_data(&self) -> (T, ComputationResult)
    where
        T: Clone,
    {
        let (d, r) = self.data();
        (d.clone(), r)
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

    pub fn map<Z, M: Fn(&T) -> Z + 'a>(&self, mapper: M) -> Signal<'a, Z> {
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

    pub fn map_pure<Z, M: Fn(&T) -> Z + 'a>(&self, mapper: M) -> Signal<'a, Z> {
        let initial = mapper(&*self.data().0);
        let this = self.clone();
        let node = self.0.world.create_node(
            move |_idx, t| {
                let (data, res) = this.data();
                if res == ComputationResult::Changed {
                    *t = mapper(&*data);
                }
                res
            },
            initial,
        );
        self.add_depending(node.0.index);
        node
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(&self, filter: F, initial: T) -> Signal<'a, T>
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

    pub fn bind<Z: Clone + 'a, B: Fn(&T) -> Signal<'a, Z> + 'a>(&self, binder: B) -> Signal<'a, Z> {
        let this = self.clone();
        let curr_data = self.data().0;
        let current_node = binder(&*curr_data);
        let cn_idx = current_node.0.index;
        let initial = current_node.data().0.clone();
        let node_store = RefCell::new(current_node);
        let node = self.0.world.create_node(
            move |idx, t| {
                let (v, mut changed) = this.data();
                if changed == ComputationResult::Changed {
                    let new_node = binder(&*v);
                    new_node.add_depending(idx);
                    node_store.replace(new_node).remove_depending(idx);
                }

                let store = node_store.borrow();
                let (v, child_changed) = store.data();
                //We have changed when the child has changed or we have a new child node, even if that one has not changed
                if child_changed == ComputationResult::Changed
                    || changed == ComputationResult::Changed
                {
                    *t = v.clone();
                    changed = ComputationResult::Changed;
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

pub struct Mutable<'a, T> {
    world: World,
    index: NodeIndex,
    value_store: Rc<RefCell<Option<T>>>,
    //Node field exists purely to ensure that the node is not dropped before the mutable
    _node: Signal<'a, T>,
}

impl<'a, T> Mutable<'a, T> {
    pub fn set(&mut self, value: T) {
        self.value_store.replace(Some(value));
        self.world.mark_dirty(self.index);
    }
}

#[cfg(test)]
mod tests {
    use crate::signals::{ComputationResult, Signal, World};
    use std::fmt::Debug;

    fn assert_changed<T: Eq + Clone + Debug>(expected: T, node: &Signal<T>) {
        let (data, changed) = node.cloned_data();
        assert_eq!(ComputationResult::Changed, changed);
        assert_eq!(expected, data);
    }

    fn assert_unchanged<T: Eq + Clone + Debug>(expected: T, node: &Signal<T>) {
        let (data, changed) = node.cloned_data();
        assert_eq!(ComputationResult::Unchanged, changed);
        assert_eq!(expected, data);
    }

    #[test]
    fn test_map1() {
        let world = World::new();
        let s1 = world.constant(3);
        let mapped = s1.map(|i| i + 2);
        assert_eq!(5, mapped.cloned_data().0);
    }

    #[test]
    fn test_map2() {
        let world = World::new();
        let (mut m1, n1) = world.mutable(2);
        let mapped = n1.map(|i| i + 2);
        assert_unchanged(4, &mapped);
        m1.set(3);
        assert_changed(5, &mapped);
        assert_unchanged(5, &mapped);
    }

    #[test]
    fn test_filter() {
        let world = World::new();
        let (mut m1, n1) = world.mutable(2);
        let filtered = n1.filter(|x| x % 2 == 0, 0);
        assert_unchanged(2, &filtered);
        m1.set(4);
        assert_changed(4, &filtered);
        m1.set(5);
        assert_unchanged(4, &filtered);
    }

    #[test]
    fn test_filter2() {
        let world = World::new();
        let (mut m1, n1) = world.mutable(1);
        let filtered = n1.filter(|x| x % 2 == 0, 0);
        assert_unchanged(0, &filtered);
        m1.set(4);
        assert_changed(4, &filtered);
    }

    #[test]
    fn test_bind() {
        let world = World::new();
        let (mut m1, n1) = world.mutable(0);
        let (mut m2, n2) = world.mutable(0);
        let (mut switch, switch_node) = world.mutable(true);
        let value = switch_node.bind(|v| if *v { n1.clone() } else { n2.clone() });
        assert_unchanged(0, &value);
        m1.set(2);
        assert_changed(2, &value);
        switch.set(false);
        assert_changed(0, &value);
        m2.set(2);
        assert_changed(2, &value);
        m1.set(5);
        assert_unchanged(2, &value);
        switch.set(true);
        assert_changed(5, &value);
    }
}