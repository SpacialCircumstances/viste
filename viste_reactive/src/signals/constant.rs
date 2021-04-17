use crate::signals::*;
use crate::Data;

pub struct Constant<T: Data> {
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
