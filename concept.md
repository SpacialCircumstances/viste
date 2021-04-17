# concepts

## Signals

- Inspired by FSharp.Data.Adaptive
- Based on easily cloneable values
- Hold a value at any time
- Lazy evaluation
- Transaction-based?
- Listeners (enqueue in transaction)?

### Ideas

- Create a reader structure for reading from a source
- signal keeps track of its readers and the last time they've read a value
- When value changes, readers can receive new values, otherwise will receive old values
- Optional storing in queue of more than one old value
- Implement lists/maps on top of this

## Event streams

- Push-based stream with moving values
- Exactly one consumer

## Observables

- Modeled after Rx
- Eager, push-based
- Based on either References or easily cloneable values
- Used as foundation for lists, maps etc. (?)