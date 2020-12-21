use std::sync::mpsc::{Sender, SendError};
use crate::pipes::{Pipe, Pipes};

pub fn send<'a, T: Copy + 'a>(sender: Sender<T>, result: Pipes<'a, Result<(), SendError<T>>>) -> Pipe<'a, T> {
    Pipe::new(move |t| {
        result.notify_all(&sender.send(*t));
    })
}

pub fn send_clone<'a, T: Clone + 'a>(sender: Sender<T>, result: Pipes<'a, Result<(), SendError<T>>>) -> Pipe<'a, T> {
    Pipe::new(move |t: &T| {
        result.notify_all(&sender.send(t.clone()));
    })
}