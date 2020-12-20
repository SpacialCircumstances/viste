use std::cell::Cell;

type Listener<'a, T> = Box<dyn Fn(&T) -> () + 'a>;

struct Listeners<'a, T>(Vec<Listener<'a, T>>);

impl<'a, T> Listeners<'a, T> {
    pub fn new() -> Self {
        Listeners(Vec::with_capacity(1))
    }

    pub fn notify_all(&self, data: &T) {
        self.0.iter().for_each(|l| (l)(data));
    }
}

fn map<'a, F, T: 'a, M: Fn(&F) -> T + 'a>(mapper: M, listeners: Listeners<'a, T>) -> Listener<'a, F> {
    return Box::new(move |f| {
        let t = mapper(f);
        listeners.notify_all(&t)
    })
}

fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a>(filter: F, listeners: Listeners<'a, T>) -> Listener<'a, T> {
    return Box::new(move |t| {
        if filter(t) {
            listeners.notify_all(t);
        }
    })
}

fn cache<'a, T: Copy + Eq + 'a>(listeners: Listeners<'a, T>) -> Listener<'a, T> {
    let mut cached: Cell<Option<T>> = Cell::new(None);
    return Box::new(move |t| {
        match &cached.get() {
            Some(old) if old == t => (),
            _ => {
                cached.replace(Some(*t));
                listeners.notify_all(t);
            }
        }
    })
}

fn callback<'a, T, F: Fn(&T) -> () + 'a>(callback: F) -> Listener<'a, T> {
    return Box::new(callback)
}

fn push<T>(value: T, listener: Listener<T>) {
    (listener)(&value)
}

fn connect<'a, T: 'a>(listeners: Listeners<'a, T>) -> Listener<'a, T> {
    return Box::new(move |t| {
        listeners.notify_all(t);
    })
}