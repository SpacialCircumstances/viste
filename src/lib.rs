trait Listener<T> {
    fn invoke(&mut self, param: &T);
}

trait Observable<T> {
    fn subscribe<>(&mut self, listener: dyn Listener<T>);
}