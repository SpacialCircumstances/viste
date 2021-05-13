use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct FromIter<'a, T: Data + 'a> {
    store: BufferedStore<T>,
    iterator: Box<dyn Iterator<Item = T> + 'a>,
    node: OwnNode,
}

impl<'a, T: Data + 'a> FromIter<'a, T> {
    pub fn new<I: Iterator<Item = T> + 'a>(world: World, iter: I) -> Self {
        Self {
            store: BufferedStore::new(),
            node: OwnNode::new(world),
            iterator: Box::new(iter),
        }
    }
}

impl<'a, T: Data + 'a> ComputationCore for FromIter<'a, T> {
    type ComputationResult = Option<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        match self.store.read(reader) {
            Some(x) => Some(x),
            None => match self.iterator.next() {
                Some(next) => {
                    self.store.push(next.cheap_clone());
                    Some(next)
                }
                None => None,
            },
        }
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
