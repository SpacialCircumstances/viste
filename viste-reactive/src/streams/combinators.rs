use crate::RStream;
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;

pub fn map<'a, T: 'a, U: 'a, M: Fn(T) -> U + 'a, I: Into<RStream<'a, U>>>(
    mapper: M,
    next: I,
) -> RStream<'a, T> {
    let next = next.into();
    RStream::new(move |t| next.push(mapper(t)))
}

pub fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a, I: Into<RStream<'a, T>>>(
    filter: F,
    next: I,
) -> RStream<'a, T> {
    let next = next.into();
    RStream::new(move |t| {
        if filter(&t) {
            next.push(t);
        }
    })
}

pub fn cache<'a, T: Copy + Eq + 'a, I: Into<RStream<'a, T>>>(next: I) -> RStream<'a, T> {
    let cached: Cell<Option<T>> = Cell::new(None);
    let next = next.into();
    RStream::new(move |t| match &cached.get() {
        Some(old) if old == &t => (),
        _ => {
            cached.replace(Some(t));
            next.push(t);
        }
    })
}

pub fn cache_clone<'a, T: Clone + Eq + 'a, I: Into<RStream<'a, T>>>(next: I) -> RStream<'a, T> {
    let cached: RefCell<Option<T>> = RefCell::new(None);
    let next = next.into();
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

pub fn cache_hash<'a, T: Hash + 'a, I: Into<RStream<'a, T>>>(next: I) -> RStream<'a, T> {
    cache_by(
        |t| {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            hasher.finish()
        },
        next,
    )
}

pub fn cache_by<'a, T: 'a, X: Eq + Copy + 'a, C: Fn(&T) -> X + 'a, I: Into<RStream<'a, T>>>(
    cache_func: C,
    next: I,
) -> RStream<'a, T> {
    let next = next.into();
    let cache = Cell::new(None);
    RStream::new(move |t| {
        let new_cache_value = cache_func(&t);
        match &cache.get() {
            Some(old) if *old == new_cache_value => (),
            _ => {
                cache.replace(Some(new_cache_value));
                next.push(t);
            }
        }
    })
}

pub fn filter_map<'a, T: 'a, U: 'a, F: Fn(T) -> Option<U> + 'a, I: Into<RStream<'a, U>>>(
    f: F,
    next: I,
) -> RStream<'a, T> {
    let next = next.into();
    RStream::new(move |t| match f(t) {
        None => (),
        Some(u) => next.push(u),
    })
}

pub fn cond<
    'a,
    T: 'a,
    F: Fn(&T) -> bool + 'a,
    I1: Into<RStream<'a, T>>,
    I2: Into<RStream<'a, T>>,
>(
    cond: F,
    if_true: I1,
    if_false: I2,
) -> RStream<'a, T> {
    let if_true = if_true.into();
    let if_false = if_false.into();
    RStream::new(move |t| match cond(&t) {
        true => if_true.push(t),
        false => if_false.push(t),
    })
}
