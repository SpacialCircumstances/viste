use crate::Data;
use std::rc::Rc;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Epoch(usize);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Id(usize);

pub struct Retriever<'a, T: Data>(&'a dyn Fn() -> T);

pub struct Transaction {
    changed_signal_ids: Vec<Id>,
}

pub trait SignalCore<T: Data> {
    fn get_data(&self) -> (Epoch, Retriever<T>);
    fn mark_changed(&self, transaction: &mut Transaction);
    fn get_id(&self) -> Id;
}

pub struct Signal<T: Data>(Rc<dyn SignalCore<T>>);
