use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Folder<'a, T: Data + 'a, V: Data + 'a, F: Fn(V, T) -> V + 'a> {
    source: ParentStreamSignal<'a, T, Option<T>, StreamReader<'a, T>>,
    store: SingleValueStore<V>,
    current_value: Option<V>,
    folder: F,
    node: NodeState,
}

impl<'a, T: Data + 'a, V: Data + 'a, F: Fn(V, T) -> V + 'a> Folder<'a, T, V, F> {
    pub fn new(world: World, source: StreamSignal<'a, T>, initial: V, folder: F) -> Self {
        let node = NodeState::new(world);
        let source = ParentStreamSignal::new(source, node.node());
        Self {
            source,
            store: SingleValueStore::new(initial.cheap_clone()),
            folder,
            current_value: Some(initial),
            node,
        }
    }
}

impl<'a, T: Data + 'a, V: Data + 'a, F: Fn(V, T) -> V + 'a> ComputationCore
    for Folder<'a, T, V, F>
{
    type ComputationResult = SingleComputationResult<V>;

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.node.clean();
            let mut changed = false;
            // self.current_value is an option, but may never be None
            // it is only an option to allow using take() and avoid unnecessary cloning during folding
            while let Some(next) = self.source.compute() {
                changed = true;
                let old_value = self
                    .current_value
                    .take()
                    .expect("Current value of folder may never be None");
                self.current_value = Some((self.folder)(old_value, next));
            }
            if changed {
                self.store.set_value(
                    self.current_value
                        .as_ref()
                        .expect("self.current_value may never be None")
                        .cheap_clone(),
                );
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
