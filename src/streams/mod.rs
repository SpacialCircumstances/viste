use crate::RStream;

pub fn map<'a, T: 'a, U: 'a, M: Fn(T) -> U + 'a>(mapper: M, next: RStream<'a, U>) -> RStream<'a, T> {
    RStream::new(move |t| next.push(mapper(t)))
}