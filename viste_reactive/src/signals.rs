use std::rc::Rc;

pub trait Data: Clone {
    fn changed(&self, other: &Self) -> bool;
}

impl<T: Clone + PartialEq> Data for T {
    fn changed(&self, other: &T) -> bool {
        self != other
    }
}

pub trait SignalCore<T: Data> {
    fn get_data(&self) -> T;
}

pub struct Signal<T: Data>(Rc<dyn SignalCore<T>>);
