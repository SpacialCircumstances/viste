use crate::signals::{Signal, World};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::mpsc::{SendError, Sender};

pub struct Event<'a, T: 'a>(Box<dyn Fn(T) + 'a>);

impl<'a, T: 'a> Event<'a, T> {
    pub fn new<F: Fn(T) + 'a>(f: F) -> Self {
        Self(Box::new(f))
    }

    pub fn push(&self, value: T) {
        (self.0)(value)
    }

    pub fn dropping() -> Self {
        Self::new(|_x| {})
    }

    pub fn cloneable(self) -> Rc<Self> {
        Rc::new(self)
    }

    pub fn send(sender: Sender<T>, result: Event<'a, Result<(), SendError<T>>>) -> Self {
        Event::new(move |t| {
            result.push(sender.send(t));
        })
    }

    pub fn store(world: &World, initial: T) -> (Self, Signal<'a, T>) {
        let (mutator, node) = world.mutable(initial);
        let mutator = RefCell::new(mutator);
        let stream = Event::new(move |t| {
            mutator.borrow_mut().set(t);
        });
        (stream, node)
    }

    pub fn mapped<F: 'a, M: Fn(F) -> T + 'a>(self, mapper: M) -> Event<'a, F> {
        map(mapper, self)
    }

    pub fn filtered<F: Fn(&T) -> bool + 'a>(self, f: F) -> Self {
        filter(f, self)
    }

    pub fn filter_mapped<U: 'a, F: Fn(U) -> Option<T> + 'a>(self, fm: F) -> Event<'a, U> {
        filter_map(fm, self)
    }

    pub fn cached(self) -> Self
    where
        T: Copy + Eq,
    {
        cache(self)
    }

    pub fn cached_clone(self) -> Self
    where
        T: Clone + Eq,
    {
        cache_clone(self)
    }

    pub fn cached_hash(self) -> Self
    where
        T: Hash,
    {
        cache_hash(self)
    }
}

impl<'a, T: 'a> From<Rc<Event<'a, T>>> for Event<'a, T> {
    fn from(l: Rc<Event<'a, T>>) -> Self {
        Event::new(move |t| l.push(t))
    }
}

pub fn map<'a, T: 'a, U: 'a, M: Fn(T) -> U + 'a, I: Into<Event<'a, U>>>(
    mapper: M,
    next: I,
) -> Event<'a, T> {
    let next = next.into();
    Event::new(move |t| next.push(mapper(t)))
}

pub fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a, I: Into<Event<'a, T>>>(
    filter: F,
    next: I,
) -> Event<'a, T> {
    let next = next.into();
    Event::new(move |t| {
        if filter(&t) {
            next.push(t);
        }
    })
}

pub fn cache<'a, T: Copy + Eq + 'a, I: Into<Event<'a, T>>>(next: I) -> Event<'a, T> {
    let cached: Cell<Option<T>> = Cell::new(None);
    let next = next.into();
    Event::new(move |t| match &cached.get() {
        Some(old) if old == &t => (),
        _ => {
            cached.replace(Some(t));
            next.push(t);
        }
    })
}

pub fn cache_clone<'a, T: Clone + Eq + 'a, I: Into<Event<'a, T>>>(next: I) -> Event<'a, T> {
    let cached: RefCell<Option<T>> = RefCell::new(None);
    let next = next.into();
    Event::new(move |t: T| {
        match cached.borrow_mut().deref_mut() {
            Some(old) if old == &t => (),
            x => {
                *x = Some(t.clone());
                next.push(t);
            }
        };
    })
}

pub fn cache_hash<'a, T: Hash + 'a, I: Into<Event<'a, T>>>(next: I) -> Event<'a, T> {
    cache_by(
        |t| {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            hasher.finish()
        },
        next,
    )
}

pub fn cache_by<'a, T: 'a, X: Eq + Copy + 'a, C: Fn(&T) -> X + 'a, I: Into<Event<'a, T>>>(
    cache_func: C,
    next: I,
) -> Event<'a, T> {
    let next = next.into();
    let cache = Cell::new(None);
    Event::new(move |t| {
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

pub fn filter_map<'a, T: 'a, U: 'a, F: Fn(T) -> Option<U> + 'a, I: Into<Event<'a, U>>>(
    f: F,
    next: I,
) -> Event<'a, T> {
    let next = next.into();
    Event::new(move |t| match f(t) {
        None => (),
        Some(u) => next.push(u),
    })
}

pub fn cond<'a, T: 'a, F: Fn(&T) -> bool + 'a, I1: Into<Event<'a, T>>, I2: Into<Event<'a, T>>>(
    cond: F,
    if_true: I1,
    if_false: I2,
) -> Event<'a, T> {
    let if_true = if_true.into();
    let if_false = if_false.into();
    Event::new(move |t| match cond(&t) {
        true => if_true.push(t),
        false => if_false.push(t),
    })
}

pub fn fold<'a, T: 'a, D: 'a, F: Fn(T, &D) -> D + 'a>(
    world: &World,
    folder: F,
    initial: D,
) -> (Event<'a, T>, Signal<'a, D>) {
    let (mut set, value) = world.mutable(initial);
    let vc = value.clone();
    let set_store = RefCell::new(set);
    (
        Event::new(move |t| {
            let new_data = vc.with_data(|d, _| folder(t, d));
            set_store.borrow_mut().set(new_data);
        }),
        value,
    )
}

#[cfg(test)]
mod tests {
    use crate::events::{
        cache, cache_clone, cache_hash, cond, filter, filter_map, fold, map, Event,
    };
    use crate::signals::World;
    use std::cell::Cell;
    use std::str::FromStr;

    #[test]
    fn test_map() {
        let world = World::new();
        let (stream, res) = Event::store(&world, None);
        let mapped = map(|x: &i32| Some(*x + 1), stream);
        assert!(res.cloned_data().0.is_none());
        mapped.push(&1);
        assert_eq!(res.cloned_data().0, Some(2));
        mapped.push(&3);
        assert_eq!(res.cloned_data().0, Some(4));
    }

    #[test]
    fn test_filter() {
        let world = World::new();
        let (stream, res) = Event::store(&world, None);
        let filtered = filter(|x| x % 2 == 0, map(|n| Some(n), stream));
        assert!(res.cloned_data().0.is_none());
        filtered.push(2);
        assert_eq!(res.cloned_data().0, Some(2));
        filtered.push(3);
        assert_eq!(res.cloned_data().0, Some(2));
    }

    #[test]
    fn test_filter_map() {
        let world = World::new();
        let (stream, res) = Event::store(&world, 0);
        let f: Event<String> = filter_map(|x: String| i32::from_str(&x).ok(), stream);
        f.push(String::from("19"));
        assert_eq!(res.cloned_data().0, 19);
        f.push(String::from("TEST"));
        assert_eq!(res.cloned_data().0, 19);
        f.push(String::from("13"));
        assert_eq!(res.cloned_data().0, 13);
    }

    #[test]
    fn test_cache() {
        let counter = Cell::new(0);
        let stream = Event::new(|_x| {
            counter.set(counter.get() + 1);
        });
        let cached = cache(stream);
        cached.push(&2);
        assert_eq!(counter.get(), 1);
        cached.push(&2);
        assert_eq!(counter.get(), 1);
        cached.push(&3);
        assert_eq!(counter.get(), 2);
        cached.push(&2);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cache_hash() {
        let counter = Cell::new(0);
        let stream = Event::new(|_x| {
            counter.set(counter.get() + 1);
        });
        let cached = cache_hash(stream);
        cached.push(&2);
        assert_eq!(counter.get(), 1);
        cached.push(&2);
        assert_eq!(counter.get(), 1);
        cached.push(&3);
        assert_eq!(counter.get(), 2);
        cached.push(&2);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cache_clone() {
        let counter = Cell::new(0);
        let stream = Event::new(|_x| {
            counter.set(counter.get() + 1);
        });
        let cached = cache_clone(stream);
        cached.push(&2);
        assert_eq!(counter.get(), 1);
        cached.push(&2);
        assert_eq!(counter.get(), 1);
        cached.push(&3);
        assert_eq!(counter.get(), 2);
        cached.push(&2);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cond() {
        let world = World::new();
        let (stream1, store1) = Event::store(&world, 0);
        let (stream2, store2) = Event::store(&world, 0);
        let cw = cond(|x| x % 2 == 0, stream1, stream2);
        cw.push(1);
        assert_eq!(store2.cloned_data().0, 1);
        cw.push(2);
        assert_eq!(store1.cloned_data().0, 2);
        assert_eq!(store2.cloned_data().0, 1);
        cw.push(0);
        assert_eq!(store2.cloned_data().0, 1);
        assert_eq!(store1.cloned_data().0, 0);
    }

    #[test]
    fn test_fold() {
        let world = World::new();
        let (folder, store) = fold(&world, |a, b| a + *b, 0);
        assert_eq!(store.cloned_data().0, 0);
        folder.push(2);
        assert_eq!(store.cloned_data().0, 2);
        folder.push(2);
        assert_eq!(store.cloned_data().0, 4);
    }
}
