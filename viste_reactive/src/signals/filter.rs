use crate::signals::*;
use crate::Data;

pub struct Filter<'a, T: Data, F: Fn(&T) -> bool + 'a> {
    source: ParentSignal<'a, T>,
    current_value: SingleValueStore<T>,
    filter: F,
    node: OwnNode,
}

impl<'a, T: Data, F: Fn(&T) -> bool + 'a> Filter<'a, T, F> {
    pub fn new(world: World, parent: Signal<'a, T>, initial: T, filter: F) -> Self {
        let node = OwnNode::new(world);
        let source = ParentSignal::new(parent, node.node());
        Self {
            source,
            filter,
            node,
            current_value: SingleValueStore::new(initial),
        }
    }
}

impl<'a, T: Data + 'a, F: Fn(&T) -> bool + 'a> SignalCore<T> for Filter<'a, T, F> {
    fn compute(&mut self, reader: ReaderToken) -> T {
        if self.node.is_dirty() {
            self.node.clean();
            let new_source = self.source.compute();
            if (self.filter)(&new_source) {
                self.current_value.set_value(new_source);
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

    fn world(&self) -> &World {
        self.node.world()
    }
}
