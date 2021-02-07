use crate::events::World;
use crate::streams::{cache, cache_clone, cache_hash, cond, filter, filter_map, map, RStream};
use std::cell::Cell;
use std::str::FromStr;

#[test]
fn test_map() {
    let world = World::new();
    let (stream, res) = RStream::store(&world, None);
    let mapped = map(|x: &i32| Some(*x + 1), stream);
    assert!(res.cloned_data().0.is_none());
    mapped.push(&1);
    assert_eq!(res.cloned_data().0, Some(2));
    mapped.push(&3);
    assert_eq!(res.cloned_data().0, Some(4));
}

#[test]
fn test_filter() {
    let world = World::new();
    let (stream, res) = RStream::store(&world, None);
    let filtered = filter(|x| x % 2 == 0, map(|n| Some(n), stream));
    assert!(res.cloned_data().0.is_none());
    filtered.push(2);
    assert_eq!(res.cloned_data().0, Some(2));
    filtered.push(3);
    assert_eq!(res.cloned_data().0, Some(2));
}

#[test]
fn test_filter_map() {
    let world = World::new();
    let (stream, res) = RStream::store(&world, 0);
    let f: RStream<String> = filter_map(|x: String| i32::from_str(&x).ok(), stream);
    f.push(String::from("19"));
    assert_eq!(res.cloned_data().0, 19);
    f.push(String::from("TEST"));
    assert_eq!(res.cloned_data().0, 19);
    f.push(String::from("13"));
    assert_eq!(res.cloned_data().0, 13);
}

#[test]
fn test_cache() {
    let counter = Cell::new(0);
    let stream = RStream::new(|_x| {
        counter.set(counter.get() + 1);
    });
    let cached = cache(stream);
    cached.push(&2);
    assert_eq!(counter.get(), 1);
    cached.push(&2);
    assert_eq!(counter.get(), 1);
    cached.push(&3);
    assert_eq!(counter.get(), 2);
    cached.push(&2);
    assert_eq!(counter.get(), 3);
}

#[test]
fn test_cache_hash() {
    let counter = Cell::new(0);
    let stream = RStream::new(|_x| {
        counter.set(counter.get() + 1);
    });
    let cached = cache_hash(stream);
    cached.push(&2);
    assert_eq!(counter.get(), 1);
    cached.push(&2);
    assert_eq!(counter.get(), 1);
    cached.push(&3);
    assert_eq!(counter.get(), 2);
    cached.push(&2);
    assert_eq!(counter.get(), 3);
}

#[test]
fn test_cache_clone() {
    let counter = Cell::new(0);
    let stream = RStream::new(|_x| {
        counter.set(counter.get() + 1);
    });
    let cached = cache_clone(stream);
    cached.push(&2);
    assert_eq!(counter.get(), 1);
    cached.push(&2);
    assert_eq!(counter.get(), 1);
    cached.push(&3);
    assert_eq!(counter.get(), 2);
    cached.push(&2);
    assert_eq!(counter.get(), 3);
}

#[test]
fn test_cond() {
    let world = World::new();
    let (stream1, store1) = RStream::store(&world, 0);
    let (stream2, store2) = RStream::store(&world, 0);
    let cw = cond(|x| x % 2 == 0, stream1, stream2);
    cw.push(1);
    assert_eq!(store2.cloned_data().0, 1);
    cw.push(2);
    assert_eq!(store1.cloned_data().0, 2);
    assert_eq!(store2.cloned_data().0, 1);
    cw.push(0);
    assert_eq!(store2.cloned_data().0, 1);
    assert_eq!(store1.cloned_data().0, 0);
}
