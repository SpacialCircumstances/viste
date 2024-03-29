use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Mapper<'a, I: Data, O: Data, M: Fn(I) -> O + 'a> {
    source: ParentValueSignal<'a, I>,
    current_value: SingleValueStore<O>,
    mapper: M,
    node: NodeState,
}

impl<'a, I: Data + 'a, O: Data + 'a, M: Fn(I) -> O + 'a> Mapper<'a, I, O, M> {
    pub fn new(world: World, source: ValueSignal<'a, I>, mapper: M) -> Self {
        let node = NodeState::new(world);
        info!("Mapper signal created: {}", node.node());
        let mut source: ParentValueSignal<I> = ParentValueSignal::new(source.0, node.node());
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

    fn world(&self) -> World {
        self.node.world().clone()
    }

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}

pub struct Mapper2<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a> {
    source1: ParentCachedValueSignal<'a, I1>,
    source2: ParentCachedValueSignal<'a, I2>,
    current_value: SingleValueStore<O>,
    mapper: M,
    node: NodeState,
}

impl<'a, I1: Data, I2: Data, O: Data, M: Fn(I1, I2) -> O + 'a> Mapper2<'a, I1, I2, O, M> {
    pub fn new(
        world: World,
        source1: ValueSignal<'a, I1>,
        source2: ValueSignal<'a, I2>,
        mapper: M,
    ) -> Self {
        let node = NodeState::new(world);
        info!("Mapper2 signal created: {}", node.node());
        let mut source1: ParentCachedValueSignal<'a, I1> =
            ParentCachedValueSignal::new(source1.0, node.node());
        let mut source2: ParentCachedValueSignal<'a, I2> =
            ParentCachedValueSignal::new(source2.0, node.node());
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

    fn world(&self) -> World {
        self.node.world().clone()
    }

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}
