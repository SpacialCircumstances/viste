use crate::readers::StreamReader;
use crate::stores::{BufferedStore, Store};
use crate::*;

pub struct CombineMapper<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a> {
    source1: ParentStreamSignal<'a, I1, Option<I1>, StreamReader<'a, I1>>,
    source2: ParentStreamSignal<'a, I2, Option<I2>, StreamReader<'a, I2>>,
    cached_value1: Option<I1>,
    cached_value2: Option<I2>,
    store: BufferedStore<O>,
    mapper: M,
    node: NodeState,
}

impl<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a>
    CombineMapper<'a, I1, I2, O, M>
{
    pub fn new(
        world: World,
        source1: StreamSignal<'a, I1>,
        source2: StreamSignal<'a, I2>,
        mapper: M,
    ) -> Self {
        let node = NodeState::new(world);
        let source1 = ParentStreamSignal::new(source1, node.node());
        let source2 = ParentStreamSignal::new(source2, node.node());
        CombineMapper {
            source1,
            source2,
            store: BufferedStore::new(),
            mapper,
            node,
            cached_value1: None,
            cached_value2: None,
        }
    }
}

impl<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, M: Fn(I1, I2) -> O + 'a> ComputationCore
    for CombineMapper<'a, I1, I2, O, M>
{
    type ComputationResult = Option<O>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            match (self.source1.compute(), self.source2.compute()) {
                (Some(v1), Some(v2)) => {
                    self.store
                        .push((self.mapper)(v1.cheap_clone(), v2.cheap_clone()));
                    self.cached_value1 = Some(v1);
                    self.cached_value2 = Some(v2);
                }
                (Some(v1), None) => {
                    match &self.cached_value2 {
                        Some(v2) => self
                            .store
                            .push((self.mapper)(v1.cheap_clone(), v2.cheap_clone())),
                        None => (),
                    }
                    self.cached_value1 = Some(v1)
                }
                (None, Some(v2)) => {
                    match &self.cached_value1 {
                        Some(v1) => self
                            .store
                            .push((self.mapper)(v1.cheap_clone(), v2.cheap_clone())),
                        None => (),
                    }
                    self.cached_value2 = Some(v2)
                }
                (None, None) => (),
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
