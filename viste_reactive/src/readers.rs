use crate::*;

pub trait Reader {
    type Result;
    type Signal;
    fn new(signal: Self::Signal) -> Self;
    fn read(&mut self) -> Self::Result;
}

pub struct ChangeReader<'a, T: Data + 'a>(ValueSignal<'a, T>, ReaderToken);

impl<'a, T: Data + 'a> Reader for ChangeReader<'a, T> {
    type Result = SingleComputationResult<T>;
    type Signal = ValueSignal<'a, T>;

    fn new(signal: ValueSignal<'a, T>) -> Self {
        let reader = signal.create_reader();
        Self(signal, reader)
    }

    fn read(&mut self) -> Self::Result {
        self.0.compute(self.1)
    }
}

impl<'a, T: Data + 'a> Drop for ChangeReader<'a, T> {
    fn drop(&mut self) {
        self.0.destroy_reader(self.1)
    }
}

pub struct CachedReader<'a, T: Data + 'a> {
    signal: ValueSignal<'a, T>,
    token: ReaderToken,
    cache: T,
}

impl<'a, T: Data + 'a> Reader for CachedReader<'a, T> {
    type Result = (bool, T);
    type Signal = ValueSignal<'a, T>;

    fn new(signal: ValueSignal<'a, T>) -> Self {
        let token = signal.create_reader();
        let initial_value = signal.compute(token).unwrap_changed();
        Self {
            signal,
            token,
            cache: initial_value,
        }
    }

    fn read(&mut self) -> (bool, T) {
        match self.signal.compute(self.token) {
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
        self.signal.destroy_reader(self.token)
    }
}

pub struct StreamReader<'a, T: Data + 'a> {
    signal: StreamSignal<'a, T>,
    token: ReaderToken,
}

pub struct StreamReaderIter<'a, T: Data + 'a>(StreamReader<'a, T>);

impl<'a, T: Data + 'a> Iterator for StreamReaderIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.read()
    }
}

impl<'a, T: Data + 'a> Reader for StreamReader<'a, T> {
    type Result = Option<T>;
    type Signal = StreamSignal<'a, T>;

    fn new(signal: Self::Signal) -> Self {
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
