use std::fmt::Debug;

mod graph;
pub mod signals;

pub trait Data: Debug {
    fn changed(&self, other: &Self) -> bool;
    fn cheap_clone(&self) -> Self;
}

//TODO: Find a way to only impl for Rc, Arc, T: Copy
impl<T: Debug + Clone + PartialEq> Data for T {
    fn changed(&self, other: &T) -> bool {
        self != other
    }

    fn cheap_clone(&self) -> Self {
        self.clone()
    }
}
