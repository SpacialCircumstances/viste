use crate::signals::*;
use crate::Data;

pub struct Mapper<'a, I: Data, O: Data, M: Fn(I) -> O + 'a> {
    source: ParentSignal<'a, I>,
    current_value: O,
    mapper: M,
    node: OwnNode,
}

impl<'a, I: Data, O: Data, M: Fn(I) -> O + 'a> Mapper<'a, I, O, M> {
    pub fn new(world: World, source: Signal<'a, I>, mapper: M) -> Self {
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

impl<'a, I: Data, O: Data, M: Fn(I) -> O + 'a> SignalCore<O> for Mapper<'a, I, O, M> {
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

pub struct Mapper2<'a, I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O + 'a> {
    source1: ParentSignal<'a, I1>,
    source2: ParentSignal<'a, I2>,
    current_value: O,
    mapper: M,
    node: OwnNode,
}

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O + 'a> Mapper2<'a, I1, I2, O, M> {
    pub fn new(world: World, source1: Signal<'a, I1>, source2: Signal<'a, I2>, mapper: M) -> Self {
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

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O + 'a> SignalCore<O>
    for Mapper2<'a, I1, I2, O, M>
{
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
