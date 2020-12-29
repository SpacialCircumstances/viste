pub mod combinators;

#[cfg(test)]
mod tests {
    use crate::wires::combinators::{
        cache, cache_clone, cache_hash, cond, filter, filter_map, map, reduce,
    };
    use crate::*;
    use std::cell::Cell;
    use std::str::FromStr;

    #[test]
    fn test_map() {
        let (wire, res) = RWire::store(None);
        let mapped = map(|x: &i32| Some(*x + 1), wire.into());
        assert!(res.data().is_none());
        mapped.run(&1);
        assert_eq!(*res.data(), Some(2));
        mapped.run(&3);
        assert_eq!(*res.data(), Some(4));
    }

    #[test]
    fn test_filter() {
        let (wire, res) = RWire::store(None);
        let filtered = filter(|x| x % 2 == 0, map(|n| Some(*n), wire.into()).into());
        assert!(res.data().is_none());
        filtered.run(&2);
        assert_eq!(*res.data(), Some(2));
        filtered.run(&3);
        assert_eq!(*res.data(), Some(2));
    }

    #[test]
    fn test_filter_map() {
        let (wire, res) = RWire::store(0);
        let f: RWire<String> = filter_map(|x: &String| i32::from_str(x).ok(), wire.into());
        f.run(&String::from("19"));
        assert_eq!(*res.data(), 19);
        f.run(&String::from("TEST"));
        assert_eq!(*res.data(), 19);
        f.run(&String::from("13"));
        assert_eq!(*res.data(), 13);
    }

    #[test]
    fn test_cache() {
        let counter = Cell::new(0);
        let wire = RWire::new(|_x| {
            counter.set(counter.get() + 1);
        });
        let cached = cache(wire.into());
        cached.run(&2);
        assert_eq!(counter.get(), 1);
        cached.run(&2);
        assert_eq!(counter.get(), 1);
        cached.run(&3);
        assert_eq!(counter.get(), 2);
        cached.run(&2);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cache_hash() {
        let counter = Cell::new(0);
        let wire = RWire::new(|_x| {
            counter.set(counter.get() + 1);
        });
        let cached = cache_hash(wire.into());
        cached.run(&2);
        assert_eq!(counter.get(), 1);
        cached.run(&2);
        assert_eq!(counter.get(), 1);
        cached.run(&3);
        assert_eq!(counter.get(), 2);
        cached.run(&2);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cache_clone() {
        let counter = Cell::new(0);
        let wire = RWire::new(|_x| {
            counter.set(counter.get() + 1);
        });
        let cached = cache_clone(wire.into());
        cached.run(&2);
        assert_eq!(counter.get(), 1);
        cached.run(&2);
        assert_eq!(counter.get(), 1);
        cached.run(&3);
        assert_eq!(counter.get(), 2);
        cached.run(&2);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cond() {
        let (wire1, store1) = RWire::store(0);
        let (wire2, store2) = RWire::store(0);
        let cw = cond(|x| x % 2 == 0, wire1.into(), wire2.into());
        cw.run(&1);
        assert_eq!(*store2.data(), 1);
        cw.run(&2);
        assert_eq!(*store1.data(), 2);
        assert_eq!(*store2.data(), 1);
        cw.run(&0);
        assert_eq!(*store2.data(), 1);
        assert_eq!(*store1.data(), 0);
    }

    #[test]
    fn test_reduce() {
        let (wire, store) = RWire::store(0);
        let incr = reduce(|x, s| *s = *s + x, 0, wire.into());
        incr.run(&1);
        assert_eq!(*store.data(), 1);
        incr.run(&1);
        assert_eq!(*store.data(), 2);
        incr.run(&5);
        assert_eq!(*store.data(), 7);
    }
}
