use crate::*;

#[derive(Debug)]
pub enum SetChange<T: Data> {
    Added(T),
    Removed(T),
    Clear,
}

impl<T: Data> Data for SetChange<T> {
    fn changed(&self, other: &Self) -> bool {
        match (self, other) {
            (SetChange::Added(v1), SetChange::Added(v2)) => v1.changed(v2),
            (SetChange::Removed(v1), SetChange::Removed(v2)) => v1.changed(v2),
            (SetChange::Clear, SetChange::Clear) => false,
            _ => true,
        }
    }

    fn cheap_clone(&self) -> Self {
        match self {
            SetChange::Added(v) => SetChange::Added(v.cheap_clone()),
            SetChange::Removed(v) => SetChange::Removed(v.cheap_clone()),
            SetChange::Clear => SetChange::Clear,
        }
    }
}

pub struct CollectionSignal<'a, T: Data + 'a>(StreamSignal<'a, SetChange<T>>);

impl<'a, T: Data + 'a> Clone for CollectionSignal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T: Data + 'a> CollectionSignal<'a, T> {
    pub fn new(stream: StreamSignal<'a, SetChange<T>>) -> Self {
        CollectionSignal(stream)
    }

    pub fn changes(&self) -> StreamSignal<'a, SetChange<T>> {
        self.0.clone()
    }
}
