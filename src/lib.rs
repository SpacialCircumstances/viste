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