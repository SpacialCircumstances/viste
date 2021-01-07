use cursive::view::View;

pub mod text_view;

pub trait RView: View {
    type Bindings;

    fn bindings(&self) -> &Self::Bindings;
}
