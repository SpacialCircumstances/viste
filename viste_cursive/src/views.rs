use cursive::view::View;

pub trait RView: View {
    type Bindings;

    fn bindings(&self) -> &Self::Bindings;
}
