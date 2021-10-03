use crate::readers::StreamReader;
use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct FilterMapper<'a, T: Data + 'a, O: Data + 'a, F: Fn(T) -> Option<O> + 'a> {
    source: ParentStreamSignal<'a, T, Option<T>, StreamReader<'a, T>>,
    store: BufferedStore<O>,
    fmap: F,
    node: NodeState,
}

impl<'a, T: Data + 'a, O: Data + 'a, F: Fn(T) -> Option<O> + 'a> FilterMapper<'a, T, O, F> {
    pub fn new(world: World, source: StreamSignal<'a, T>, fmap: F) -> Self {
        let node = NodeState::new(world);
        Self {
            source: ParentStreamSignal::new(source, node.node()),
            store: BufferedStore::new(),
            fmap,
            node,
        }
    }
}

impl<'a, T: Data + 'a, O: Data + 'a, F: Fn(T) -> Option<O> + 'a> ComputationCore
    for FilterMapper<'a, T, O, F>
{
    type ComputationResult = Option<O>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            while let Some(t) = self.source.compute() {
                if let Some(v) = (self.fmap)(t) {
                    self.store.push(v)
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
