# Viste

Experimenting with reactive programming in Rust.

## Events

An `Event` is a push-based, eager representation of occurrences of values over time.

## Signals

A `Signal` is a value that can change over time. Signals are implemented as a graph of reference-counted nodes and changes are propagated by a mixed push-pull system.
When changing a node, its children are eagerly marked dirty (if necessary), but computations only happen when a value is pulled from a node.
