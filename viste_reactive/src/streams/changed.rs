use crate::*;

pub struct Changed<'a, T: Data + 'a> {
    source: ParentValueSignal<'a, T, SingleComputationResult<T>, ChangeReader<'a, T>>,
    store: BufferedStore<T>,
    node: OwnNode,
}

impl<'a, T: Data + 'a> Changed<'a, T> {
    pub fn new(world: World, source: ValueSignal<'a, T>) -> Self {
        let node = OwnNode::new(world);
        Self {
            source: ParentValueSignal::new(source, node.node()),
            store: BufferedStore::new(),
            node,
        }
    }
}

impl<'a, T: Data + 'a> ComputationCore for Changed<'a, T> {
    type ComputationResult = Option<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            if let SingleComputationResult::Changed(new) = self.source.compute() {
                self.store.push(new)
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