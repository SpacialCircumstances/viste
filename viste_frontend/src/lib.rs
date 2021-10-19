use quetta::Text;
use viste_reactive::collections::VecView;
use viste_reactive::*;
use web_sys::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute(Text, ValueSignal<'static, Text>);

impl Attribute {
    pub fn new(key: Text, value: ValueSignal<'static, Text>) -> Self {
        Self(key, value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum El {
    Text(Text),
    Element(
        VecView<'static, Attribute>,
        VecView<'static, ValueSignal<'static, El>>,
    ),
}

pub struct Component<Params>(Box<dyn Fn(Params) -> ValueSignal<'static, El>>);

impl<Params> Component<Params> {
    pub fn new<F: Fn(Params) -> ValueSignal<'static, El> + 'static>(render: F) -> Self {
        Self(Box::new(render))
    }
}

pub fn render(main: Component<()>, to: Element) {}
