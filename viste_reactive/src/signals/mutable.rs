use crate::signals::*;
use crate::Data;

pub struct Mutable<T: Data> {
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

impl<T: Data> SignalCore<T> for Mutable<T> {
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
