use quetta::Text;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use viste_reactive::collections::{CollectionSignal, SetChange, VecView};
use viste_reactive::graph::NodeIndex;
use viste_reactive::readers::StreamReader;
use viste_reactive::*;
use web_sys::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute(Text, Text);

impl Attribute {
    pub fn new(key: Text, value: Text) -> Self {
        Self(key, value)
    }
}

pub struct ReactiveHtml<'a> {
    attributes: Collector<'a, SetChange<Attribute>>,
    attributes_signal: ParentStreamSignal<
        'a,
        SetChange<Attribute>,
        Option<SetChange<Attribute>>,
        StreamReader<'a, SetChange<Attribute>>,
    >,
    children: Collector<'a, SetChange<HtmlSignal<'a>>>,
    children_signal: ParentStreamSignal<
        'a,
        SetChange<HtmlSignal<'a>>,
        Option<SetChange<HtmlSignal<'a>>>,
        StreamReader<'a, SetChange<HtmlSignal<'a>>>,
    >,
    node: NodeState,
    dom: Element,
}

impl<'a> ReactiveHtml<'a> {
    pub fn new(
        world: World,
        attributes: CollectionSignal<'a, Attribute>,
        children: CollectionSignal<'a, HtmlSignal<'a>>,
        dom: Element,
    ) -> Self {
        let node = NodeState::new(world);
        ReactiveHtml {
            attributes_signal: ParentStreamSignal::new(attributes.changes(), node.node()),
            children_signal: ParentStreamSignal::new(children.changes(), node.node()),
            attributes: attributes.changes().collect(),
            children: children.changes().collect(),
            node,
            dom,
        }
    }

    fn reconcile_attr(&self, change: &SetChange<Attribute>) {
        match change {
            SetChange::Clear => {
                let attrs = self.dom.get_attribute_names();
                for attr in attrs.iter().filter_map(|a| a.as_string()) {
                    self.dom.remove_attribute(&attr);
                }
            }
            SetChange::Removed(attr) => {
                self.dom.remove_attribute(attr.0.as_str());
            }
            SetChange::Added(attr) => {
                self.dom.set_attribute(attr.0.as_str(), attr.1.as_str());
            }
        }
    }

    fn reconcile_child(&self, change: &SetChange<HtmlSignal<'a>>) {}
}

impl<'a> ComputationCore for ReactiveHtml<'a> {
    type ComputationResult = ();

    fn compute(&mut self, reader: ReaderToken) -> Self::ComputationResult {
        if self.is_dirty() {
            self.attributes.update();
            for attr_change in self.attributes.iter() {
                self.reconcile_attr(attr_change);
            }
            self.attributes.clear();

            self.children.update();
            for children_change in self.children.iter() {
                self.reconcile_child(children_change);
            }
            self.children.clear()
        }
    }

    fn create_reader(&mut self) -> ReaderToken {
        ReaderToken::default()
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

    fn node(&self) -> NodeIndex {
        todo!()
    }
}

pub struct HtmlSignal<'a>(Rc<RefCell<dyn ComputationCore<ComputationResult = ()> + 'a>>);

impl<'a> Clone for HtmlSignal<'a> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a> PartialEq for HtmlSignal<'a> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<'a> Eq for HtmlSignal<'a> {}
