use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct Portal<T: Data> {
    store: BufferedStore<T>,
    node: NodeState,
}

impl<'a, T: Data + 'a> Portal<T> {
    pub fn new(world: World) -> Self {
        Portal {
            store: BufferedStore::new(),
            node: NodeState::new(world),
        }
    }

    pub fn send(&mut self, value: T) {
        self.store.push(value);
        self.node.mark_dirty();
    }
}

impl<'a, T: Data + 'a> ComputationCore for Portal<T> {
    type ComputationResult = Option<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        self.node.clean();
        self.store.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.store.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.store.destroy_reader(reader)
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

    fn world(&self) -> &World {
        self.node.world()
    }
}
