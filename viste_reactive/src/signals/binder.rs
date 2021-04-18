use crate::signals::*;
use crate::Data;

pub struct Binder<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> {
    binder: B,
    current_signal: ParentSignal<'a, O>,
    node: OwnNode,
    parent: ParentSignal<'a, I>,
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> Binder<'a, I, O, B> {
    pub fn new(world: World, parent: Signal<'a, I>, binder: B) -> Self {
        let node = OwnNode::new(world);
        info!("Binder signal created: {}", node.node());
        let parent = ParentSignal::new(parent, node.node());
        let initial_signal = ParentSignal::new(binder(&parent.compute()), node.node());
        Binder {
            binder,
            node,
            parent,
            current_signal: initial_signal,
        }
    }
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> SignalCore<O>
    for Binder<'a, I, O, B>
{
    fn compute(&mut self, reader: ReaderToken) -> O {
        if self.node.is_dirty() {
            self.node.clean();
            let new_source = self.parent.compute();
            self.current_signal = ParentSignal::new((self.binder)(&new_source), self.node.node());
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
