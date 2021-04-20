use crate::signals::*;
use crate::Data;

pub struct Binder<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> Signal<'a, O> + 'a> {
    binder: B,
    current_signal: ParentSignal<'a, O, SingleComputationResult<O>, ChangeReader<'a, O>>,
    parent: ParentSignal<'a, I, SingleComputationResult<I>, ChangeReader<'a, I>>,
    current_value: SingleValueStore<O>,
    node: OwnNode,
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> Signal<'a, O> + 'a> Binder<'a, I, O, B> {
    pub fn new(world: World, parent: Signal<'a, I>, binder: B) -> Self {
        let node = OwnNode::new(world);
        info!("Binder signal created: {}", node.node());
        let mut parent: ParentSignal<I, SingleComputationResult<I>, ChangeReader<I>> =
            ParentSignal::new(parent, node.node());
        let mut initial_signal: ParentSignal<O, SingleComputationResult<O>, ChangeReader<O>> =
            ParentSignal::new(binder(parent.compute().unwrap_changed()), node.node());
        let current_value = SingleValueStore::new(initial_signal.compute().unwrap_changed());
        Binder {
            binder,
            node,
            parent,
            current_value,
            current_signal: initial_signal,
        }
    }
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> Signal<'a, O> + 'a> ComputationCore<O>
    for Binder<'a, I, O, B>
{
    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<O> {
        if self.node.is_dirty() {
            self.node.clean();
            if let SingleComputationResult::Changed(new_source) = self.parent.compute() {
                self.current_signal =
                    ParentSignal::new((self.binder)(new_source), self.node.node());
            }
            if let SingleComputationResult::Changed(new_value) = self.current_signal.compute() {
                self.current_value.set_value(new_value)
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
