use std::cell::{Cell, RefCell};
use std::ops::DerefMut;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::*;

pub fn map<'a, F: 'a, T: 'a, M: Fn(&F) -> T + 'a>(mapper: M, pipes: RWires<'a, T>) -> RWire<'a, F> {
    RWire::new(move |f| {
        let t = mapper(f);
        pipes.distribute(&t)
    })
}

pub fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a>(filter: F, pipes: RWires<'a, T>) -> RWire<'a, T> {
    RWire::new(move |t| {
        if filter(t) {
            pipes.distribute(t);
        }
    })
}

pub fn cache<'a, T: Copy + Eq + 'a>(pipes: RWires<'a, T>) -> RWire<'a, T> {
    let cached: Cell<Option<T>> = Cell::new(None);
    RWire::new(move |t| {
        match &cached.get() {
            Some(old) if old == t => (),
            _ => {
                cached.replace(Some(*t));
                pipes.distribute(t);
            }
        }
    })
}

pub fn cache_clone<'a, T: Clone + Eq + 'a>(pipes: RWires<'a, T>) -> RWire<'a, T> {
    let cached: RefCell<Option<T>> = RefCell::new(None);
    RWire::new(move |t| {
        match cached.borrow_mut().deref_mut() {
            Some(old) if old == t => (),
            x => {
                *x = Some(t.clone());
                pipes.distribute(t);
            }
        };
    })
}

pub fn cache_hash<'a, T: Hash + 'a>(pipes: RWires<'a, T>) -> RWire<'a, T> {
    let cached: Cell<Option<u64>> = Cell::new(None);
    RWire::new(move |t: &T| {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        let new_hash = hasher.finish();
        match cached.get() {
            Some(old_hash) if old_hash == new_hash => (),
            _ => {
                cached.set(Some(new_hash));
                pipes.distribute(t);
            }
        }
    })
}

pub fn reduce<'a, T: 'a, S: 'a, F: Fn(&T, &mut S) -> () + 'a>(reducer: F, initial: S, out: RWires<'a, S>) -> RWire<'a, T> {
    let state = RefCell::new(initial);
    RWire::new(move |t: &T| {
        let mut s = state.borrow_mut();
        reducer(t, &mut s);
        drop(s);
        out.distribute(&*state.borrow());
    })
}

pub fn copied<'a, T: Copy + 'a>(pipes: RWires<'a, T>) -> RWire<'a, &T> {
    RWire::new(move |t| {
        pipes.distribute(*t);
    })
}

pub fn filter_map<'a, T: 'a, U: 'a, F: Fn(&T) -> Option<U> + 'a>(f: F, wires: RWires<'a, U>) -> RWire<'a, T> {
    RWire::new(move |t| {
        match f(t) {
            None => (),
            Some(u) => wires.distribute(&u)
        }
    })
}

pub fn cond<'a, T: 'a, F: Fn(&T) -> bool + 'a>(cond: F, if_true: RWires<'a, T>, if_false: RWires<'a, T>) -> RWire<'a, T> {
    RWire::new(move |t| {
        match cond(t) {
            true => if_true.distribute(t),
            false => if_false.distribute(t)
        }
    })
}