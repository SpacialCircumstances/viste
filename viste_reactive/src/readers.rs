use crate::*;

pub trait Reader<'a, CR> {
    type Result;
    fn new(signal: Signal<'a, CR>) -> Self;
    fn read(&mut self) -> Self::Result;
}

pub struct ChangeReader<'a, T: Data + 'a>(ValueSignal<'a, T>, ReaderToken);

impl<'a, T: Data + 'a> Reader<'a, SingleComputationResult<T>> for ChangeReader<'a, T> {
    type Result = SingleComputationResult<T>;

    fn new(signal: Signal<'a, SingleComputationResult<T>>) -> Self {
        let reader = signal.create_reader();
        Self(ValueSignal::new(signal), reader)
    }

    fn read(&mut self) -> Self::Result {
        self.0.signal().compute(self.1)
    }
}

impl<'a, T: Data + 'a> Drop for ChangeReader<'a, T> {
    fn drop(&mut self) {
        self.0.signal().destroy_reader(self.1)
    }
}

pub struct CachedReader<'a, T: Data + 'a> {
    signal: ValueSignal<'a, T>,
    token: ReaderToken,
    cache: T,
}

impl<'a, T: Data + 'a> Reader<'a, SingleComputationResult<T>> for CachedReader<'a, T> {
    type Result = (bool, T);

    fn new(signal: Signal<'a, SingleComputationResult<T>>) -> Self {
        let token = signal.create_reader();
        let initial_value = signal.compute(token).unwrap_changed();
        Self {
            signal: ValueSignal::new(signal),
            token,
            cache: initial_value,
        }
    }

    fn read(&mut self) -> (bool, T) {
        match self.signal.signal().compute(self.token) {
            SingleComputationResult::Changed(new_v) => {
                self.cache = new_v;
                (true, self.cache.cheap_clone())
            }
            SingleComputationResult::Unchanged => (false, self.cache.cheap_clone()),
        }
    }
}

impl<'a, T: Data + 'a> Drop for CachedReader<'a, T> {
    fn drop(&mut self) {
        self.signal.signal().destroy_reader(self.token)
    }
}

pub struct StreamReader<'a, T: Data + 'a> {
    signal: Signal<'a, Option<T>>,
    token: ReaderToken,
}

pub struct StreamReaderIter<'a, T: Data + 'a>(StreamReader<'a, T>);

impl<'a, T: Data + 'a> Iterator for StreamReaderIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.read()
    }
}

impl<'a, T: Data + 'a> Reader<'a, Option<T>> for StreamReader<'a, T> {
    type Result = Option<T>;

    fn new(signal: Signal<'a, Self::Result>) -> Self {
        let token = signal.create_reader();
        Self { signal, token }
    }

    fn read(&mut self) -> Self::Result {
        self.signal.compute(self.token)
    }
}

impl<'a, T: Data + 'a> IntoIterator for StreamReader<'a, T> {
    type Item = T;
    type IntoIter = StreamReaderIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        StreamReaderIter(self)
    }
}

impl<'a, T: Data + 'a> Drop for StreamReader<'a, T> {
    fn drop(&mut self) {
        self.signal.destroy_reader(self.token)
    }
}
