use crate::*;

pub struct Mapper<'a, I: Data, O: Data, M: Fn(I) -> O + 'a> {
    source: ParentValueSignal<'a, I, SingleComputationResult<I>, ChangeReader<'a, I>>,
    current_value: SingleValueStore<O>,
    mapper: M,
    node: OwnNode,
}

impl<'a, I: Data + 'a, O: Data + 'a, M: Fn(I) -> O + 'a> Mapper<'a, I, O, M> {
    pub fn new(world: World, source: ValueSignal<'a, I>, mapper: M) -> Self {
        let node = OwnNode::new(world);
        info!("Mapper signal created: {}", node.node());
        let mut source: ParentValueSignal<I, SingleComputationResult<I>, ChangeReader<I>> =
            ParentValueSignal::new(source, node.node());
        let current_value = SingleValueStore::new(mapper(source.compute().unwrap_changed()));
        Mapper {
            source,
            mapper,
            current_value,
            node,
        }
    }
}

impl<'a, I: Data + 'a, O: Data + 'a, M: Fn(I) -> O + 'a> ComputationCore for Mapper<'a, I, O, M> {
    type ComputationResult = SingleComputationResult<O>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<O> {
        if self.node.is_dirty() {
            self.node.clean();
            if let SingleComputationResult::Changed(new_source) = self.source.compute() {
                self.current_value.set_value((self.mapper)(new_source));
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

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}

pub struct Mapper2<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a> {
    source1: ParentValueSignal<'a, I1, (bool, I1), CachedReader<'a, I1>>,
    source2: ParentValueSignal<'a, I2, (bool, I2), CachedReader<'a, I2>>,
    current_value: SingleValueStore<O>,
    mapper: M,
    node: OwnNode,
}

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(I1, I2) -> O + 'a> Mapper2<'a, I1, I2, O, M> {
    pub fn new(
        world: World,
        source1: ValueSignal<'a, I1>,
        source2: ValueSignal<'a, I2>,
        mapper: M,
    ) -> Self {
        let node = OwnNode::new(world);
        info!("Mapper2 signal created: {}", node.node());
        let mut source1: ParentValueSignal<'a, I1, (bool, I1), CachedReader<'a, I1>> =
            ParentValueSignal::new(source1, node.node());
        let mut source2: ParentValueSignal<'a, I2, (bool, I2), CachedReader<'a, I2>> =
            ParentValueSignal::new(source2, node.node());
        let initial_value = mapper(source1.compute().1, source2.compute().1);
        Self {
            mapper,
            node,
            current_value: SingleValueStore::new(initial_value),
            source1,
            source2,
        }
    }
}

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(I1, I2) -> O + 'a> ComputationCore
    for Mapper2<'a, I1, I2, O, M>
{
    type ComputationResult = SingleComputationResult<O>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<O> {
        if self.node.is_dirty() {
            self.node.clean();
            let (changed1, v1) = self.source1.compute();
            let (changed2, v2) = self.source2.compute();
            if changed1 || changed2 {
                self.current_value.set_value((self.mapper)(v1, v2))
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

    fn is_dirty(&self) -> bool {
        self.node.is_dirty()
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}
