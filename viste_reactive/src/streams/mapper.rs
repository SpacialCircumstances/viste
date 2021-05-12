use crate::graph::NodeIndex;
use crate::readers::StreamReader;
use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct Mapper<'a, T: Data + 'a, R: Data + 'a, M: Fn(T) -> R + 'a> {
    source: ParentStreamSignal<'a, T, Option<T>, StreamReader<'a, T>>,
    values: BufferedStore<R>,
    mapper: M,
    own_node: OwnNode,
}

impl<'a, T: Data + 'a, R: Data + 'a, M: Fn(T) -> R + 'a> Mapper<'a, T, R, M> {
    pub fn new(world: World, source: StreamSignal<'a, T>, mapper: M) -> Self {
        let own_node = OwnNode::new(world);
        Mapper {
            source: ParentStreamSignal::new(source, own_node.node()),
            values: BufferedStore::new(),
            mapper,
            own_node,
        }
    }
}

impl<'a, T: Data + 'a, R: Data + 'a, M: Fn(T) -> R + 'a> ComputationCore for Mapper<'a, T, R, M> {
    type ComputationResult = Option<R>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.own_node.is_dirty() {
            self.own_node.clean();
            while let Some(next) = self.source.compute() {
                self.values.push((self.mapper)(next))
            }
        }
        self.values.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.values.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.values.destroy_reader(reader)
    }

    fn add_dependency(&mut self, child: NodeIndex) {
        self.own_node.add_dependency(child)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.own_node.remove_dependency(child)
    }

    fn is_dirty(&self) -> bool {
        self.own_node.is_dirty()
    }

    fn world(&self) -> &World {
        self.own_node.world()
    }
}
