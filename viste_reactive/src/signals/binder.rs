use crate::signals::*;
use crate::Data;

pub struct Binder<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> {
    binder: B,
    current_signal: Signal<'a, O>,
    node: OwnNode,
    parent: ParentSignal<'a, I>,
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> Binder<'a, I, O, B> {
    pub fn new(world: World, parent: Signal<'a, I>, binder: B) -> Self {
        let node = OwnNode::new(world);
        let parent = ParentSignal::new(parent, node.node());
        let initial_signal = binder(&parent.compute());
        initial_signal.add_dependency(node.node());
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
            self.current_signal.remove_dependency(self.node.node());
            self.current_signal = (self.binder)(&new_source);
            self.current_signal.add_dependency(self.node.node());
        }
        self.current_signal.compute(reader)
    }

    fn add_dependency(&mut self, child: NodeIndex) -> ReaderToken {
        self.node.add_dependency(child);
        ReaderToken(0)
    }

    fn remove_dependency(&mut self, child: NodeIndex) {
        self.node.remove_dependency(child)
    }

    fn world(&self) -> &World {
        self.node.world()
    }
}

impl<'a, I: Data + 'a, O: Data + 'a, B: Fn(&I) -> Signal<'a, O> + 'a> Drop for Binder<'a, I, O, B> {
    fn drop(&mut self) {
        self.current_signal.remove_dependency(self.node.node())
    }
}
