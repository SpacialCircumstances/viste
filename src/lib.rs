trait Listener<T> {
    fn invoke(&mut self, param: &T);
}

trait Observable<T> {
    fn subscribe<>(&mut self, listener: dyn Listener<T>);
}

impl<T, F: Fn(&T) -> ()> Listener<T> for F {
    fn invoke(&mut self, param: &T) {
        (self)(param)
    }
}