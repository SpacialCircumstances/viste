use crate::signals::*;
use crate::Data;

pub struct Binder<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> {
    binder: B,
    current_signal: ParentSignal<'a, O, SingleComputationResult<O>, ChangeReader<'a, O>>,
    parent: ParentSignal<'a, I, SingleComputationResult<I>, ChangeReader<'a, I>>,
    node: OwnNode,
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> Binder<'a, I, O, B> {
    pub fn new(world: World, parent: Signal<'a, I>, binder: B) -> Self {
        let node = OwnNode::new(world);
        info!("Binder signal created: {}", node.node());
        let mut parent: ParentSignal<I, SingleComputationResult<I>, ChangeReader<I>> =
            ParentSignal::new(parent, node.node());
        let initial_signal: ParentSignal<O, SingleComputationResult<O>, ChangeReader<O>> =
            ParentSignal::new(binder(&parent.compute().unwrap_changed()), node.node());
        Binder {
            binder,
            node,
            parent,
            current_signal: initial_signal,
        }
    }
}

//TODO: ADD READER STUFF
impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> ComputationCore<O>
    for Binder<'a, I, O, B>
{
    fn compute(&mut self, reader: ReaderToken) -> SingleComputationResult<O> {
        if self.node.is_dirty() {
            self.node.clean();
            match self.parent.compute() {
                SingleComputationResult::Unchanged => (),
                SingleComputationResult::Changed(t) => {
                    self.current_signal = ParentSignal::new((self.binder)(&t), self.node.node());
                }
            }
        }
        self.current_signal.compute()
    }

    fn create_reader(&mut self) -> ReaderToken {
        ReaderToken(0)
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {}

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
