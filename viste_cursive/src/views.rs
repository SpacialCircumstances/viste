use cursive::view::View;

pub trait RView: View {
    type Bindings;

    fn get_bindings(&self) -> &Self::Bindings;
}
