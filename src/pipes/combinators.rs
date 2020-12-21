use std::cell::{Cell, RefCell};
use std::ops::DerefMut;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::pipes::{Pipe, Pipes};

pub fn map<'a, F, T: 'a, M: Fn(&F) -> T + 'a>(mapper: M, pipes: Pipes<'a, T>) -> Pipe<'a, F> {
    Pipe::new(move |f| {
        let t = mapper(f);
        pipes.notify_all(&t)
    })
}

pub fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a>(filter: F, pipes: Pipes<'a, T>) -> Pipe<'a, T> {
    Pipe::new(move |t| {
        if filter(t) {
            pipes.notify_all(t);
        }
    })
}

pub fn cache<'a, T: Copy + Eq + 'a>(pipes: Pipes<'a, T>) -> Pipe<'a, T> {
    let cached: Cell<Option<T>> = Cell::new(None);
    Pipe::new(move |t| {
        match &cached.get() {
            Some(old) if old == t => (),
            _ => {
                cached.replace(Some(*t));
                pipes.notify_all(t);
            }
        }
    })
}

pub fn cache_clone<'a, T: Clone + Eq + 'a>(pipes: Pipes<'a, T>) -> Pipe<'a, T> {
    let cached: RefCell<Option<T>> = RefCell::new(None);
    Pipe::new(move |t| {
        match cached.borrow_mut().deref_mut() {
            Some(old) if old == t => (),
            x => {
                *x = Some(t.clone());
                pipes.notify_all(t);
            }
        };
    })
}

pub fn cache_hash<'a, T: Hash + 'a>(pipes: Pipes<'a, T>) -> Pipe<'a, T> {
    let cached: Cell<Option<u64>> = Cell::new(None);
    Pipe::new(move |t: &T| {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        let new_hash = hasher.finish();
        match cached.get() {
            Some(old_hash) if old_hash == new_hash => (),
            _ => {
                cached.set(Some(new_hash));
                pipes.notify_all(t);
            }
        }
    })
}

pub fn copied<'a, T: Copy + 'a>(pipes: Pipes<'a, T>) -> Pipe<'a, &T> {
    Pipe::new(move |t| {
        pipes.notify_all(*t);
    })
}