use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Filter<'a, T: Data, F: Fn(&T) -> bool + 'a> {
    source: ParentValueSignal<'a, T>,
    current_value: SingleValueStore<T>,
    filter: F,
    node: NodeState,
}

impl<'a, T: Data, F: Fn(&T) -> bool + 'a> Filter<'a, T, F> {
    pub fn new(world: World, parent: ValueSignal<'a, T>, initial: T, filter: F) -> Self {
        let node = NodeState::new(world);
        info!("Filter signal created: {}", node.node());
        let source = ParentValueSignal::new(parent, node.node());
        Self {
            source,
            filter,
            node,
            current_value: SingleValueStore::new(initial),
        }
    }
}

impl<'a, T: Data + 'a, F: Fn(&T) -> bool + 'a> ComputationCore for Filter<'a, T, F> {
    type ComputationResult = SingleComputationResult<T>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<T> {
        if self.node.is_dirty() {
            self.node.clean();
            if let SingleComputationResult::Changed(new_source) = self.source.compute() {
                if (self.filter)(&new_source) {
                    self.current_value.set_value(new_source);
                }
            }
        }
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
