use crate::signals::*;
use crate::Data;

pub struct Constant<T: Data> {
    node: OwnNode,
    value: SingleValueStore<T>,
}

impl<T: Data> Constant<T> {
    pub fn new(world: World, value: T) -> Self {
        let node = OwnNode::new(world);
        info!("Constant signal created: {}", node.node());
        Self {
            node,
            value: SingleValueStore::new(value),
        }
    }
}

impl<T: Data> SignalCore<T> for Constant<T> {
    fn compute(&mut self, reader: ReaderToken) -> T {
        self.value.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.value.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.value.destroy_reader(reader)
    }

    fn add_dependency(&mut self, child: NodeIndex) {}

    fn remove_dependency(&mut self, child: NodeIndex) {}

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}
