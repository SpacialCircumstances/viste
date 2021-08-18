use crate::readers::StreamReader;
use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct Cached<'a, T: Data + 'a> {
    source: ParentStreamSignal<'a, T, Option<T>, StreamReader<'a, T>>,
    last: Option<T>,
    store: BufferedStore<T>,
    node: NodeState,
}

impl<'a, T: Data + 'a> Cached<'a, T> {
    pub fn new(world: World, source: StreamSignal<'a, T>) -> Self {
        let node = NodeState::new(world);
        let source = ParentStreamSignal::new(source, node.node());
        Self {
            source,
            last: None,
            store: BufferedStore::new(),
            node,
        }
    }
}

impl<'a, T: Data + 'a> ComputationCore for Cached<'a, T> {
    type ComputationResult = Option<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            while let Some(next) = self.source.compute() {
                if let Some(last) = self.last.take() {
                    if next.changed(&last) {
                        self.store.push(next.cheap_clone())
                    }
                    self.last = Some(next)
                } else {
                    self.last = Some(next.cheap_clone());
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
}
