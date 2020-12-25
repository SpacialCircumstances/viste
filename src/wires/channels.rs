use std::sync::mpsc::{Sender, SendError};
use crate::*;

pub fn send<'a, T: Copy + 'a>(sender: Sender<T>, result: RWires<'a, Result<(), SendError<T>>>) -> RWire<'a, T> {
    RWire::new(move |t| {
        result.distribute(&sender.send(*t));
    })
}

pub fn send_clone<'a, T: Clone + 'a>(sender: Sender<T>, result: RWires<'a, Result<(), SendError<T>>>) -> RWire<'a, T> {
    RWire::new(move |t: &T| {
        result.distribute(&sender.send(t.clone()));
    })
}