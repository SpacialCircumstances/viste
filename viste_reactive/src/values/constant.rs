use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Constant<T: Data> {
    node: OwnNode,
    value: SingleValueStore<T>,
}

impl<T: Data> Constant<T> {
    pub fn new(world: World, value: T) -> Self {
        let node = OwnNode::new(world);
        info!("Constant signal created: {}", node.node());
        Self {
            node,
            value: SingleValueStore::new(value),
        }
    }
}

impl<T: Data> ComputationCore for Constant<T> {
    type ComputationResult = SingleComputationResult<T>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<T> {
        self.value.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.value.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.value.destroy_reader(reader)
    }

    fn add_dependency(&mut self, _child: NodeIndex) {}

    fn remove_dependency(&mut self, _child: NodeIndex) {}

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}
