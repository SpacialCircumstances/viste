pub mod events;
mod graph;
pub mod old;
pub mod signals;

pub trait Data {
    fn changed(&self, other: &Self) -> bool;
    fn cheap_clone(&self) -> Self;
}

impl<T: Copy + PartialEq> Data for T {
    fn changed(&self, other: &T) -> bool {
        self != other
    }

    fn cheap_clone(&self) -> Self {
        *self
    }
}
