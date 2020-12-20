use std::cell::Cell;

type Listener<'a, T> = Box<dyn Fn(&T) -> () + 'a>;

fn map<'a, F, T: 'a, M: Fn(&F) -> T + 'a>(mapper: M, listeners: Vec<Listener<'a, T>>) -> Listener<'a, F> {
    return Box::new(move |f| {
        let t = mapper(f);
        for l in &listeners {
            (l)(&t)
        }
    })
}

fn filter<'a, T: 'a, F: Fn(&T) -> bool + 'a>(filter: F, listeners: Vec<Listener<'a, T>>) -> Listener<'a, T> {
    return Box::new(move |t| {
        if filter(t) {
            for l in &listeners {
                (l)(t)
            }
        }
    })
}

fn cache<'a, T: Copy + Eq + 'a>(listeners: Vec<Listener<'a, T>>) -> Listener<'a, T> {
    let mut cached: Cell<Option<T>> = Cell::new(None);
    return Box::new(move |t| {
        match &cached.get() {
            Some(old) if old == t => (),
            _ => {
                cached.replace(Some(*t));
                for l in &listeners {
                    (l)(t)
                }
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

fn connect<'a, T: 'a>(listeners: Vec<Listener<'a, T>>) -> Listener<'a, T> {
    return Box::new(move |t| {
        for l in &listeners {
            (l)(t)
        }
    })
}