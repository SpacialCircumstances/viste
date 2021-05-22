use crate::*;

#[derive(Debug)]
pub enum ListChange<T: Data> {
    Push(T),
    Remove(usize),
    Insert(T, usize),
    Clear,
    Swap(usize, usize),
}

impl<T: Data> Data for ListChange<T> {
    fn changed(&self, other: &Self) -> bool {
        match (self, other) {
            (ListChange::Push(v1), ListChange::Push(v2)) => v1.changed(v2),
            (ListChange::Insert(v1, i1), ListChange::Insert(v2, i2)) => i1 != i2 || v1.changed(v2),
            (ListChange::Clear, ListChange::Clear) => false,
            (ListChange::Swap(li1, li2), ListChange::Swap(ri1, ri2)) => li1 != ri1 || li2 != ri2,
            (ListChange::Remove(i1), ListChange::Remove(i2)) => i1 != i2,
            _ => true,
        }
    }

    fn cheap_clone(&self) -> Self {
        match self {
            ListChange::Push(v) => ListChange::Push(v.cheap_clone()),
            ListChange::Insert(v, i) => ListChange::Insert(v.cheap_clone(), *i),
            ListChange::Remove(i) => ListChange::Remove(*i),
            ListChange::Clear => ListChange::Clear,
            ListChange::Swap(i1, i2) => ListChange::Swap(*i1, *i2),
        }
    }
}

pub struct ListSignal<'a, T: Data + 'a>(StreamSignal<'a, ListChange<T>>);

impl<'a, T: Data + 'a> Clone for ListSignal<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub fn lift<A: Data, B: Data, F: Fn(A) -> B>(func: F) -> impl Fn(ListChange<A>) -> ListChange<B> {
    move |t| match t {
        ListChange::Push(v) => ListChange::Push(func(v)),
        ListChange::Insert(v, i) => ListChange::Insert(func(v), i),
        ListChange::Clear => ListChange::Clear,
        ListChange::Remove(i) => ListChange::Remove(i),
        ListChange::Swap(i1, i2) => ListChange::Swap(i1, i2),
    }
}

impl<'a, T: Data + 'a> ListSignal<'a, T> {
    pub fn map<O: Data + 'a, M: Fn(T) -> O + 'a>(&self, mapper: M) -> ListSignal<'a, O> {
        ListSignal(self.0.map(lift(mapper)))
    }
}
