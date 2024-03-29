use crate::stores::{SingleValueStore, Store};
use crate::*;
use log::info;

pub struct Mutable<T: Data> {
    current_value: SingleValueStore<T>,
    node: NodeState,
}

impl<T: Data> Mutable<T> {
    pub fn new(world: World, initial: T) -> Self {
        let node = NodeState::new(world);
        info!("Mutable signal created: {}", node.node());
        Self {
            current_value: SingleValueStore::new(initial),
            node,
        }
    }

    pub fn set(&mut self, value: T) {
        self.current_value.set_value(value);
        self.node.mark_dirty(DirtyingCause::External)
    }
}

impl<T: Data> ComputationCore for Mutable<T> {
    type ComputationResult = SingleComputationResult<T>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<T> {
        self.node.clean();
        self.current_value.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.current_value.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.current_value.destroy_reader(reader)
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.remove_dependency(child)
    }

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> World {
        self.node.world().clone()
    }

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}
