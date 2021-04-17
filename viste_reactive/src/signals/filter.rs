use crate::signals::*;
use crate::Data;

pub struct Filter<T: Data, F: Fn(&T) -> bool> {
    source: ParentSignal<T>,
    current_value: T,
    filter: F,
    node: OwnNode,
}

impl<T: Data, F: Fn(&T) -> bool> Filter<T, F> {
    pub fn new(world: World, parent: Signal<T>, initial: T, filter: F) -> Self {
        let node = OwnNode::new(world);
        let source = ParentSignal::new(parent, node.node());
        Self {
            source,
            filter,
            node,
            current_value: initial,
        }
    }
}

impl<T: Data, F: Fn(&T) -> bool> SignalCore<T> for Filter<T, F> {
    fn compute(&mut self) -> T {
        if self.node.is_dirty() {
            self.node.clean();
            let new_source = self.source.compute();
            if (self.filter)(&new_source) {
                self.current_value = new_source;
            }
        }
        self.current_value.cheap_clone()
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
