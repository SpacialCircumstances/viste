use std::sync::mpsc::{Sender, SendError};
use crate::{Pipe, Pipes};

pub fn send<'a, T: Copy + 'a>(sender: Sender<T>, result: Pipes<'a, Result<(), SendError<T>>>) -> Pipe<'a, T> {
    Pipe::new(move |t| {
        result.distribute(&sender.send(*t));
    })
}

pub fn send_clone<'a, T: Clone + 'a>(sender: Sender<T>, result: Pipes<'a, Result<(), SendError<T>>>) -> Pipe<'a, T> {
    Pipe::new(move |t: &T| {
        result.distribute(&sender.send(t.clone()));
    })
}