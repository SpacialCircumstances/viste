use crate::streams::RStream;

pub enum ListDelta<T> {
    Add(T),
    Remove(T),
    Clear,
    Swap(usize, usize),
    Insert(usize, T),
    RemoveAt(usize),
}

pub type ListStream<'a, T> = RStream<'a, ListDelta<T>>;

pub struct StreamingListSender<'a, T>(ListStream<'a, T>);

impl<'a, T> StreamingListSender<'a, T> {
    pub fn new(stream: ListStream<'a, T>) -> Self {
        Self(stream)
    }

    pub fn push(&mut self, value: T) {
        self.0.push(ListDelta::Add(value));
    }

    pub fn clear(&mut self) {
        self.0.push(ListDelta::Clear);
    }

    pub fn remove(&mut self, value: T) {
        self.0.push(ListDelta::Remove(value));
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.0.push(ListDelta::Insert(index, element));
    }

    pub fn remove_at(&mut self, index: usize) {
        self.0.push(ListDelta::RemoveAt(index))
    }

    pub fn swap(&mut self, i1: usize, i2: usize) {
        self.0.push(ListDelta::Swap(i1, i2))
    }
}
