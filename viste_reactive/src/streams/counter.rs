use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Counter<'a, T: Data + 'a> {
    source: ParentStreamSignal<'a, T>,
    value: SingleValueStore<u64>,
    node: NodeState,
}

impl<'a, T: Data + 'a> Counter<'a, T> {
    pub fn new(world: World, source: StreamSignal<'a, T>) -> Self {
        let node = NodeState::new(world);
        Self {
            source: ParentSignal::new(source, node.node()),
            value: SingleValueStore::new(0),
            node,
        }
    }
}

impl<'a, T: Data + 'a> ComputationCore for Counter<'a, T> {
    type ComputationResult = SingleComputationResult<u64>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();

            let mut current = self.value.get();

            while self.source.compute().is_some() {
                current += 1;
            }

            self.value.set_value(current);
        }

        self.value.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.value.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.value.destroy_reader(reader)
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
