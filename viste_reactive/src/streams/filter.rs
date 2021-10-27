use crate::readers::StreamReader;
use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct Filter<'a, T: Data, F: Fn(&T) -> bool + 'a> {
    source: ParentStreamSignal<'a, Option<T>, StreamSignal<'a, T>, StreamReader<'a, T>>,
    store: BufferedStore<T>,
    filter: F,
    node: NodeState,
}

impl<'a, T: Data, F: Fn(&T) -> bool + 'a> Filter<'a, T, F> {
    pub fn new(world: World, source: StreamSignal<'a, T>, filter: F) -> Self {
        let node = NodeState::new(world);
        let source = ParentStreamSignal::new(source, node.node());
        Self {
            source,
            store: BufferedStore::new(),
            filter,
            node,
        }
    }
}

impl<'a, T: Data, F: Fn(&T) -> bool + 'a> ComputationCore for Filter<'a, T, F> {
    type ComputationResult = Option<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            while let Some(next) = self.source.compute() {
                if (self.filter)(&next) {
                    self.store.push(next)
                }
            }
        }
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

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}
