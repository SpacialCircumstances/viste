use crate::readers::StreamReader;
use crate::stores::{BufferedStore, Store};
use crate::*;
use std::collections::HashMap;

pub struct Many<'a, T: Data + 'a> {
    sources: HashMap<
        NodeIndex,
        ParentStreamSignal<'a, Option<T>, StreamSignal<'a, T>, Option<T>, StreamReader<'a, T>>,
    >,
    values: BufferedStore<T>,
    node: NodeState,
}

impl<'a, T: Data + 'a> Many<'a, T> {
    pub fn new(world: World, sources: Vec<StreamSignal<'a, T>>) -> Self {
        let node = NodeState::new(world);
        let sources = sources
            .into_iter()
            .map(|signal| (signal.node(), ParentStreamSignal::new(signal, node.node())))
            .collect();
        Many {
            node,
            sources,
            values: BufferedStore::new(),
        }
    }
}

impl<'a, T: Data + 'a> ComputationCore for Many<'a, T> {
    type ComputationResult = Option<T>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        let dirty_state = self.node.reset_dirty_state();
        match dirty_state {
            DirtyFlag::Basic(false) => (),
            DirtyFlag::Basic(true) => {
                for (_, source) in self.sources.iter_mut() {
                    while let Some(val) = source.compute() {
                        self.values.push(val)
                    }
                }
            }
            DirtyFlag::Changed(changed) => {
                for changed_node in changed {
                    let source = &mut self
                        .sources
                        .get_mut(&changed_node)
                        .expect("Dirtied parent node not found");
                    while let Some(val) = source.compute() {
                        self.values.push(val)
                    }
                }
            }
        }

        self.values.read(reader)
    }

    fn create_reader(&mut self) -> ReaderToken {
        self.values.create_reader()
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.values.destroy_reader(reader)
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

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}
