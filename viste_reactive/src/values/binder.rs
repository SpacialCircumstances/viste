use crate::readers::{CachedReader, ChangeReader};
use crate::stores::{SingleValueStore, Store};
use crate::*;

pub struct Binder<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> ValueSignal<'a, O> + 'a> {
    binder: B,
    current_signal: ParentValueSignal<'a, O, SingleComputationResult<O>, ChangeReader<'a, O>>,
    parent: ParentValueSignal<'a, I, SingleComputationResult<I>, ChangeReader<'a, I>>,
    current_value: SingleValueStore<O>,
    node: OwnNode,
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(I) -> ValueSignal<'a, O> + 'a> Binder<'a, I, O, B> {
    pub fn new(world: World, parent: ValueSignal<'a, I>, binder: B) -> Self {
        let node = OwnNode::new(world);
        info!("Binder signal created: {}", node.node());
        let mut parent: ParentValueSignal<I, SingleComputationResult<I>, ChangeReader<I>> =
            ParentValueSignal::new(parent, node.node());
        let mut initial_signal: ParentValueSignal<O, SingleComputationResult<O>, ChangeReader<O>> =
            ParentValueSignal::new(binder(parent.compute().unwrap_changed()), node.node());
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
                self.current_signal.set_parent((self.binder)(new_source));
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

pub struct Binder2<
    'a,
    I1: Data + 'a,
    I2: Data + 'a,
    O: Data + 'a,
    B: Fn(I1, I2) -> ValueSignal<'a, O> + 'a,
> {
    binder: B,
    current_signal: ParentValueSignal<'a, O, SingleComputationResult<O>, ChangeReader<'a, O>>,
    parent1: ParentValueSignal<'a, I1, (bool, I1), CachedReader<'a, I1>>,
    parent2: ParentValueSignal<'a, I2, (bool, I2), CachedReader<'a, I2>>,
    current_value: SingleValueStore<O>,
    node: OwnNode,
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
        let node = OwnNode::new(world);
        info!("Binder2 signal created: {}", node.node());
        let mut parent1: ParentValueSignal<I1, (bool, I1), CachedReader<'a, I1>> =
            ParentValueSignal::new(parent1, node.node());
        let mut parent2: ParentValueSignal<I2, (bool, I2), CachedReader<'a, I2>> =
            ParentValueSignal::new(parent2, node.node());
        let mut initial_signal: ParentValueSignal<O, SingleComputationResult<O>, ChangeReader<O>> =
            ParentValueSignal::new(
                binder(parent1.compute().1, parent2.compute().1),
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
                self.current_signal.set_parent((self.binder)(s1, s2));
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
