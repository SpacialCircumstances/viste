pub mod events;
mod graph;
pub mod old;
pub mod signals;

pub trait Data: 'static {
    fn changed(&self, other: &Self) -> bool;
    fn cheap_clone(&self) -> Self;
}

impl<T: Copy + PartialEq + 'static> Data for T {
    fn changed(&self, other: &T) -> bool {
        self != other
    }

    fn cheap_clone(&self) -> Self {
        *self
    }
}
