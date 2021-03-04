# Viste

Viste (Quenya for "change") is a simple library for reactive programming with Rust. Viste is focused on an efficient implementation of reactive programming and tries to avoid unnecessary cloning and recomputation.

## Events

An `Event` is a push-based, eager representation of occurrences of values over time.

## Signals

A `Signal` is a value that can change over time. Signals are implemented as a graph of reference-counted nodes and changes are propagated by a mixed push-pull system.
When changing a node, its children are eagerly marked dirty (if necessary), but computations only happen when a value is pulled from a node.
