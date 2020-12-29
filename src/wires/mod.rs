pub mod combinators;

#[cfg(test)]
mod tests {
    use crate::*;
    use crate::wires::combinators::map;

    #[test]
    fn test_map() {
        let (wire, res) = RWire::store(None);
        let mapped = map(|x| Some(x + 1), wire.into());
        assert!(res.data().is_none());
        mapped.run(&1);
        assert_eq!(*res.data(), Some(2));
        mapped.run(&3);
        assert_eq!(*res.data(), Some(4));
    }
}