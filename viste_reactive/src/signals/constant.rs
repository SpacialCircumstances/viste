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

impl<T: Data> SignalCore<T> for Constant<T> {
    fn compute(&mut self, reader: ReaderToken) -> T {
        self.value.cheap_clone()
    }

    fn add_dependency(&mut self, child: NodeIndex) -> ReaderToken {
        ReaderToken(0)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {}

    fn world(&self) -> &World {
        self.node.world()
    }
}
