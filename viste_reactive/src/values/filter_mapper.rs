use crate::readers::ChangeReader;
use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct FilterMapper<'a, T: Data + 'a, O: Data + 'a, F: Fn(T) -> Option<O> + 'a> {
    source: ParentValueSignal<'a, T, SingleComputationResult<T>, ChangeReader<'a, T>>,
    store: SingleValueStore<O>,
    fmap: F,
    node: NodeState,
}

impl<'a, T: Data + 'a, O: Data + 'a, F: Fn(T) -> Option<O> + 'a> FilterMapper<'a, T, O, F> {
    pub fn new(world: World, source: ValueSignal<'a, T>, initial: O, fmap: F) -> Self {
        let node = NodeState::new(world);
        Self {
            source: ParentValueSignal::new(source, node.node()),
            store: SingleValueStore::new(initial),
            fmap,
            node,
        }
    }
}

impl<'a, T: Data + 'a, O: Data + 'a, F: Fn(T) -> Option<O> + 'a> ComputationCore
    for FilterMapper<'a, T, O, F>
{
    type ComputationResult = SingleComputationResult<O>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            if let SingleComputationResult::Changed(t) = self.source.compute() {
                if let Some(v) = (self.fmap)(t) {
                    self.store.set_value(v)
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
