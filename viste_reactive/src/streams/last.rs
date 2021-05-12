use crate::readers::StreamReader;
use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Last<'a, T: Data + 'a> {
    source: ParentStreamSignal<'a, T, Option<T>, StreamReader<'a, T>>,
    value: SingleValueStore<T>,
    node: OwnNode,
}

impl<'a, T: Data + 'a> Last<'a, T> {
    pub fn new(world: World, source: StreamSignal<'a, T>, initial: T) -> Self {
        let node = OwnNode::new(world);
        Self {
            source: ParentStreamSignal::new(source, node.node()),
            value: SingleValueStore::new(initial),
            node,
        }
    }
}

impl<'a, T: Data + 'a> ComputationCore for Last<'a, T> {
    type ComputationResult = SingleComputationResult<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            let mut last = None;
            while let Some(nv) = self.source.compute() {
                last = Some(nv);
            }
            match last {
                None => (),
                Some(l) => self.value.set_value(l),
            }
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

    fn world(&self) -> &World {
        self.node.world()
    }
}
