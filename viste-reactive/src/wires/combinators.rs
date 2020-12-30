use crate::*;
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;

pub fn map<'a, F: 'a, T: 'a, M: Fn(&F) -> T + 'a, I: Into<RWires<'a, T>>>(
    mapper: M,
    wires: I,
) -> RWire<'a, F> {
    let wires = wires.into();
    RWire::new(move |f| {
        let t = mapper(f);
        wires.distribute(&t)
    })
}

pub fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a, I: Into<RWires<'a, T>>>(
    filter: F,
    wires: I,
) -> RWire<'a, T> {
    let wires = wires.into();
    RWire::new(move |t| {
        if filter(t) {
            wires.distribute(t);
        }
    })
}

pub fn cache<'a, T: Copy + Eq + 'a, I: Into<RWires<'a, T>>>(wires: I) -> RWire<'a, T> {
    let cached: Cell<Option<T>> = Cell::new(None);
    let wires = wires.into();
    RWire::new(move |t| match &cached.get() {
        Some(old) if old == t => (),
        _ => {
            cached.replace(Some(*t));
            wires.distribute(t);
        }
    })
}

pub fn cache_clone<'a, T: Clone + Eq + 'a, I: Into<RWires<'a, T>>>(wires: I) -> RWire<'a, T> {
    let cached: RefCell<Option<T>> = RefCell::new(None);
    let wires = wires.into();
    RWire::new(move |t| {
        match cached.borrow_mut().deref_mut() {
            Some(old) if old == t => (),
            x => {
                *x = Some(t.clone());
                wires.distribute(t);
            }
        };
    })
}

pub fn cache_hash<'a, T: Hash + 'a, I: Into<RWires<'a, T>>>(wires: I) -> RWire<'a, T> {
    cache_by(
        |t| {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            hasher.finish()
        },
        wires,
    )
}

pub fn cache_by<'a, T: 'a, X: Eq + Copy + 'a, C: Fn(&T) -> X + 'a, I: Into<RWires<'a, T>>>(
    cache_func: C,
    wires: I,
) -> RWire<'a, T> {
    let wires = wires.into();
    let cache = Cell::new(None);
    RWire::new(move |t| {
        let new_cache_value = cache_func(t);
        match &cache.get() {
            Some(old) if *old == new_cache_value => (),
            _ => {
                cache.replace(Some(new_cache_value));
                wires.distribute(t);
            }
        }
    })
}

pub fn reduce<'a, T: 'a, S: 'a, F: Fn(&T, &mut S) + 'a, I: Into<RWires<'a, S>>>(
    reducer: F,
    initial: S,
    out: I,
) -> RWire<'a, T> {
    let out = out.into();
    let state = RefCell::new(initial);
    RWire::new(move |t: &T| {
        let mut s = state.borrow_mut();
        reducer(t, &mut s);
        drop(s);
        out.distribute(&*state.borrow());
    })
}

pub fn filter_map<'a, T: 'a, U: 'a, F: Fn(&T) -> Option<U> + 'a, I: Into<RWires<'a, U>>>(
    f: F,
    wires: I,
) -> RWire<'a, T> {
    let wires = wires.into();
    RWire::new(move |t| match f(t) {
        None => (),
        Some(u) => wires.distribute(&u),
    })
}

pub fn cond<'a, T: 'a, F: Fn(&T) -> bool + 'a, I1: Into<RWires<'a, T>>, I2: Into<RWires<'a, T>>>(
    cond: F,
    if_true: I1,
    if_false: I2,
) -> RWire<'a, T> {
    let if_true = if_true.into();
    let if_false = if_false.into();
    RWire::new(move |t| match cond(t) {
        true => if_true.distribute(t),
        false => if_false.distribute(t),
    })
}
