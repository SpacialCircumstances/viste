use crate::RStream;

pub enum ListDelta<T> {
    Add(T),
    Remove(T),
    Clear,
    Swap(usize, usize),
    Insert(usize, T),
    RemoveAt(usize),
}

pub type ListStream<'a, T> = RStream<'a, ListDelta<T>>;
