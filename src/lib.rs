use std::cell::{Cell, RefCell};
use std::ops::DerefMut;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

struct Listener<'a, T>(Box<dyn Fn(&T) -> () + 'a>);

impl<'a, T> Listener<'a, T> {
    pub fn new<F: Fn(&T) -> () + 'a>(fun: F) -> Self {
        Listener(Box::new(fun))
    }

    pub fn invoke(&self, data: &T) {
        (self.0)(data)
    }
}

struct Listeners<'a, T>(Vec<Listener<'a, T>>);

impl<'a, T> Listeners<'a, T> {
    pub fn new() -> Self {
        Listeners(Vec::with_capacity(1))
    }

    pub fn notify_all(&self, data: &T) {
        self.0.iter().for_each(|l| l.invoke(data));
    }
}

fn map<'a, F, T: 'a, M: Fn(&F) -> T + 'a>(mapper: M, listeners: Listeners<'a, T>) -> Listener<'a, F> {
    Listener::new(move |f| {
        let t = mapper(f);
        listeners.notify_all(&t)
    })
}

fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a>(filter: F, listeners: Listeners<'a, T>) -> Listener<'a, T> {
    Listener::new(move |t| {
        if filter(t) {
            listeners.notify_all(t);
        }
    })
}

fn cache<'a, T: Copy + Eq + 'a>(listeners: Listeners<'a, T>) -> Listener<'a, T> {
    let mut cached: Cell<Option<T>> = Cell::new(None);
    Listener::new(move |t| {
        match &cached.get() {
            Some(old) if old == t => (),
            _ => {
                cached.replace(Some(*t));
                listeners.notify_all(t);
            }
        }
    })
}

fn cache_clone<'a, T: Clone + Eq + 'a>(listeners: Listeners<'a, T>) -> Listener<'a, T> {
    let mut cached: RefCell<Option<T>> = RefCell::new(None);
    Listener::new(move |t| {
       match cached.borrow_mut().deref_mut() {
           Some(old) if old == t => (),
           x => {
               *x = Some(t.clone());
               listeners.notify_all(t);
           }
       };
    })
}

fn cache_hash<'a, T: Hash + 'a>(listeners: Listeners<'a, T>) -> Listener<'a, T> {
    let mut cached: Cell<Option<u64>> = Cell::new(None);
    Listener::new(move |t: &T| {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        let new_hash = hasher.finish();
        match cached.get() {
            Some(old_hash) if old_hash == new_hash => (),
            _ => {
                cached.set(Some(new_hash));
                listeners.notify_all(t);
            }
        }
    })
}

fn callback<'a, T, F: Fn(&T) -> () + 'a>(callback: F) -> Listener<'a, T> {
    Listener::new(callback)
}

fn connect<'a, T: 'a>(listeners: Listeners<'a, T>) -> Listener<'a, T> {
    Listener::new(move |t| {
        listeners.notify_all(t);
    })
}