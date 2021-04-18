use crate::signals::*;
use crate::Data;

pub struct Mapper<'a, I: Data, O: Data, M: Fn(I) -> O + 'a> {
    source: ParentSignal<'a, I>,
    current_value: SingleValueStore<O>,
    mapper: M,
    node: OwnNode,
}

impl<'a, I: Data + 'a, O: Data + 'a, M: Fn(I) -> O + 'a> Mapper<'a, I, O, M> {
    pub fn new(world: World, source: Signal<'a, I>, mapper: M) -> Self {
        let node = OwnNode::new(world);
        info!("Mapper signal created: {}", node.node());
        let source = ParentSignal::new(source, node.node());
        let current_value = SingleValueStore::new(mapper(source.compute()));
        Mapper {
            source,
            mapper,
            current_value,
            node,
        }
    }
}

impl<'a, I: Data + 'a, O: Data + 'a, M: Fn(I) -> O + 'a> SignalCore<O> for Mapper<'a, I, O, M> {
    fn compute(&mut self, reader: ReaderToken) -> O {
        if self.node.is_dirty() {
            self.current_value
                .set_value((self.mapper)(self.source.compute()));
            self.node.clean();
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

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}

pub struct Mapper2<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(&I1, &I2) -> O + 'a> {
    source1: ParentSignal<'a, I1>,
    source2: ParentSignal<'a, I2>,
    current_value: SingleValueStore<O>,
    mapper: M,
    node: OwnNode,
}

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O + 'a> Mapper2<'a, I1, I2, O, M> {
    pub fn new(world: World, source1: Signal<'a, I1>, source2: Signal<'a, I2>, mapper: M) -> Self {
        let node = OwnNode::new(world);
        info!("Mapper2 signal created: {}", node.node());
        let source1 = ParentSignal::new(source1, node.node());
        let source2 = ParentSignal::new(source2, node.node());
        let initial_value = mapper(&source1.compute(), &source2.compute());
        Self {
            mapper,
            node,
            current_value: SingleValueStore::new(initial_value),
            source1,
            source2,
        }
    }
}

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(&I1, &I2) -> O + 'a> SignalCore<O>
    for Mapper2<'a, I1, I2, O, M>
{
    fn compute(&mut self, reader: ReaderToken) -> O {
        if self.node.is_dirty() {
            self.node.clean();
            self.current_value.set_value((self.mapper)(
                &self.source1.compute(),
                &self.source2.compute(),
            ));
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

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}
