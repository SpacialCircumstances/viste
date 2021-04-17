use crate::signals::*;
use crate::Data;

pub struct Mapper<I: Data, O: Data, M: Fn(I) -> O> {
    source: ParentSignal<I>,
    current_value: O,
    mapper: M,
    node: OwnNode,
}

impl<I: Data, O: Data, M: Fn(I) -> O> Mapper<I, O, M> {
    pub fn new(world: World, source: Signal<I>, mapper: M) -> Self {
        let current_value = mapper(source.compute());
        let node = OwnNode::new(world);
        Mapper {
            source: ParentSignal::new(source, node.node()),
            mapper,
            current_value,
            node,
        }
    }
}

impl<I: Data, O: Data, M: Fn(I) -> O> SignalCore<O> for Mapper<I, O, M> {
    fn compute(&mut self) -> O {
        if self.node.is_dirty() {
            self.current_value = (self.mapper)(self.source.compute());
            self.node.clean();
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

pub struct Mapper2<I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O> {
    source1: ParentSignal<I1>,
    source2: ParentSignal<I2>,
    current_value: O,
    mapper: M,
    node: OwnNode,
}

impl<I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O> Mapper2<I1, I2, O, M> {
    pub fn new(world: World, source1: Signal<I1>, source2: Signal<I2>, mapper: M) -> Self {
        let node = OwnNode::new(world);
        let initial_value = mapper(&source1.compute(), &source2.compute());
        let source1 = ParentSignal::new(source1, node.node());
        let source2 = ParentSignal::new(source2, node.node());
        Self {
            mapper,
            node,
            current_value: initial_value,
            source1,
            source2,
        }
    }
}

impl<I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O> SignalCore<O> for Mapper2<I1, I2, O, M> {
    fn compute(&mut self) -> O {
        if self.node.is_dirty() {
            self.node.clean();
            self.current_value = (self.mapper)(&self.source1.compute(), &self.source2.compute())
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
