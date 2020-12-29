use crate::RStream;
use std::cell::{RefCell, Cell};
use std::ops::DerefMut;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub fn map<'a, T: 'a, U: 'a, M: Fn(T) -> U + 'a>(mapper: M, next: RStream<'a, U>) -> RStream<'a, T> {
    RStream::new(move |t| next.push(mapper(t)))
}

pub fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a>(filter: F, next: RStream<'a, T>) -> RStream<'a, T> {
    RStream::new(move |t| {
        if filter(&t) {
            next.push(t);
        }
    })
}

pub fn cache<'a, T: Copy + Eq + 'a>(next: RStream<'a, T>) -> RStream<'a, T> {
    let cached: Cell<Option<T>> = Cell::new(None);
    RStream::new(move |t| {
        match &cached.get() {
            Some(old) if old == &t => (),
            _ => {
                cached.replace(Some(t));
                next.push(t);
            }
        }
    })
}

pub fn cache_clone<'a, T: Clone + Eq + 'a>(next: RStream<'a, T>) -> RStream<'a, T> {
    let cached: RefCell<Option<T>> = RefCell::new(None);
    RStream::new(move |t: T| {
        match cached.borrow_mut().deref_mut() {
            Some(old) if old == &t => (),
            x => {
                *x = Some(t.clone());
                next.push(t);
            }
        };
    })
}

pub fn cache_hash<'a, T: Hash + 'a>(next: RStream<'a, T>) -> RStream<'a, T> {
    let cached: Cell<Option<u64>> = Cell::new(None);
    RStream::new(move |t: T| {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        let new_hash = hasher.finish();
        match cached.get() {
            Some(old_hash) if old_hash == new_hash => (),
            _ => {
                cached.set(Some(new_hash));
                next.push(t);
            }
        }
    })
}

pub fn filter_map<'a, T: 'a, U: 'a, F: Fn(T) -> Option<U> + 'a>(f: F, next: RStream<'a, U>) -> RStream<'a, T> {
    RStream::new(move |t| {
        match f(t) {
            None => (),
            Some(u) => next.push(u)
        }
    })
}

pub fn cond<'a, T: 'a, F: Fn(&T) -> bool + 'a>(cond: F, if_true: RStream<'a, T>, if_false: RStream<'a, T>) -> RStream<'a, T> {
    RStream::new(move |t| {
        match cond(&t) {
            true => if_true.push(t),
            false => if_false.push(t)
        }
    })
}