use crate::*;

pub struct ZipMapper<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a> {
    source1: ParentStreamSignal<'a, I1, Option<I1>, StreamReader<'a, I1>>,
    source2: ParentStreamSignal<'a, I2, Option<I2>, StreamReader<'a, I2>>,
    cached_value1: Option<I1>,
    cached_value2: Option<I2>,
    store: BufferedStore<O>,
    mapper: M,
    node: OwnNode,
}

impl<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a>
    ZipMapper<'a, I1, I2, O, M>
{
    pub fn new(
        world: World,
        source1: StreamSignal<'a, I1>,
        source2: StreamSignal<'a, I2>,
        mapper: M,
    ) -> Self {
        let node = OwnNode::new(world);
        let mut source1 = ParentStreamSignal::new(source1, node.node());
        let mut source2 = ParentStreamSignal::new(source2, node.node());
        Self {
            cached_value1: None,
            cached_value2: None,
            store: BufferedStore::new(),
            source1,
            source2,
            mapper,
            node,
        }
    }
}

impl<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a> ComputationCore
    for ZipMapper<'a, I1, I2, O, M>
{
    type ComputationResult = Option<O>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            loop {
                let value1 = self.cached_value1.take().or_else(|| self.source1.compute());
                let value2 = self.cached_value2.take().or_else(|| self.source2.compute());
                match (value1, value2) {
                    (Some(v1), Some(v2)) => {
                        self.store.push((self.mapper)(v1, v2));
                    }
                    (Some(v1), None) => {
                        self.cached_value1 = Some(v1);
                        break;
                    }
                    (None, Some(v2)) => {
                        self.cached_value2 = Some(v2);
                        break;
                    }
                    (None, None) => break,
                }
            }
        }
        self.store.read(reader)
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
