use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Binder<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> ValueSignal<'a, O> + 'a> {
    binder: B,
    current_signal: ParentValueSignal<'a, O>,
    parent: ParentValueSignal<'a, I>,
    current_value: SingleValueStore<O>,
    node: NodeState,
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> ValueSignal<'a, O> + 'a> Binder<'a, I, O, B> {
    pub fn new(world: World, parent: ValueSignal<'a, I>, binder: B) -> Self {
        let node = NodeState::new(world);
        info!("Binder signal created: {}", node.node());
        let mut parent: ParentValueSignal<I> = ParentValueSignal::new(parent.0, node.node());
        let mut initial_signal: ParentValueSignal<O> =
            ParentValueSignal::new(binder(parent.compute().unwrap_changed()).0, node.node());
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

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> ValueSignal<'a, O> + 'a> ComputationCore
    for Binder<'a, I, O, B>
{
    type ComputationResult = SingleComputationResult<O>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<O> {
        if self.node.is_dirty() {
            self.node.clean();
            if let SingleComputationResult::Changed(new_source) = self.parent.compute() {
                let new_signal = (self.binder)(new_source);
                self.current_signal.set_parent(new_signal.0);
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

    fn world(&self) -> World {
        self.node.world().clone()
    }

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}

pub struct Binder2<
    'a,
    I1: Data + 'a,
    I2: Data + 'a,
    O: Data + 'a,
    B: Fn(I1, I2) -> ValueSignal<'a, O> + 'a,
> {
    binder: B,
    current_signal: ParentValueSignal<'a, O>,
    parent1: ParentCachedValueSignal<'a, I1>,
    parent2: ParentCachedValueSignal<'a, I2>,
    current_value: SingleValueStore<O>,
    node: NodeState,
}

impl<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, B: Fn(I1, I2) -> ValueSignal<'a, O> + 'a>
    Binder2<'a, I1, I2, O, B>
{
    pub fn new(
        world: World,
        parent1: ValueSignal<'a, I1>,
        parent2: ValueSignal<'a, I2>,
        binder: B,
    ) -> Self {
        let node = NodeState::new(world);
        info!("Binder2 signal created: {}", node.node());
        let mut parent1: ParentCachedValueSignal<I1> =
            ParentCachedValueSignal::new(parent1.0, node.node());
        let mut parent2: ParentCachedValueSignal<I2> =
            ParentCachedValueSignal::new(parent2.0, node.node());
        let mut initial_signal: ParentValueSignal<O> = ParentValueSignal::new(
            binder(parent1.compute().1, parent2.compute().1).0,
            node.node(),
        );
        let current_value = SingleValueStore::new(initial_signal.compute().unwrap_changed());
        Binder2 {
            binder,
            node,
            parent1,
            parent2,
            current_value,
            current_signal: initial_signal,
        }
    }
}

impl<'a, I1: Data + 'a, I2: Data + 'a, O: Data + 'a, B: Fn(I1, I2) -> ValueSignal<'a, O> + 'a>
    ComputationCore for Binder2<'a, I1, I2, O, B>
{
    type ComputationResult = SingleComputationResult<O>;

    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<O> {
        if self.node.is_dirty() {
            self.node.clean();
            let (changed1, s1) = self.parent1.compute();
            let (changed2, s2) = self.parent2.compute();
            if changed1 || changed2 {
                let new_signal = (self.binder)(s1, s2);
                self.current_signal.set_parent(new_signal.0);
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

    fn world(&self) -> World {
        self.node.world().clone()
    }

    fn node(&self) -> NodeIndex {
        self.node.node()
    }
}
