use crate::*;

//Should this also contain the read method and the type?
pub trait Store {
    fn create_reader(&mut self) -> ReaderToken;
    fn destroy_reader(&mut self, reader: ReaderToken);
}

pub struct SingleValueStore<T: Data> {
    value: T,
    reader_states: Slab<bool>,
}

impl<T: Data> SingleValueStore<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            reader_states: Slab::new(),
        }
    }

    pub fn set_value(&mut self, value: T) {
        if !value.changed(&self.value) {
            return;
        }

        self.value = value;
        self.reader_states
            .iter_mut()
            .for_each(|(_, rs)| *rs = false);
    }

    pub fn read(&mut self, reader: ReaderToken) -> SingleComputationResult<T> {
        let state = self
            .reader_states
            .get_mut(reader.0)
            .expect("Reader not found");
        if !*state {
            *state = true;
            SingleComputationResult::Changed(self.value.cheap_clone())
        } else {
            SingleComputationResult::Unchanged
        }
    }

    pub fn get(&self) -> T {
        self.value.cheap_clone()
    }
}

impl<T: Data> Store for SingleValueStore<T> {
    fn create_reader(&mut self) -> ReaderToken {
        let reader = self.reader_states.insert(false);
        ReaderToken(reader)
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.reader_states.remove(reader.0);
    }
}

pub struct BufferedStore<T: Data> {
    //TODO: Optimize with single queue and position as state?
    reader_states: Slab<VecDeque<T>>,
}

impl<T: Data> BufferedStore<T> {
    pub fn new() -> Self {
        Self {
            reader_states: Slab::new(),
        }
    }

    pub fn read(&mut self, reader: ReaderToken) -> Option<T> {
        self.reader_states[reader.0].pop_front()
    }

    pub fn push(&mut self, value: T) {
        self.reader_states
            .iter_mut()
            .for_each(|(_, rs)| rs.push_back(value.cheap_clone()))
    }
}

impl<T: Data> Default for BufferedStore<T> {
    fn default() -> Self {
        BufferedStore::new()
    }
}

impl<T: Data> Store for BufferedStore<T> {
    fn create_reader(&mut self) -> ReaderToken {
        ReaderToken(self.reader_states.insert(VecDeque::new()))
    }

    fn destroy_reader(&mut self, reader: ReaderToken) {
        self.reader_states.remove(reader.0);
    }
}
