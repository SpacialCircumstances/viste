use crate::RStream;

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