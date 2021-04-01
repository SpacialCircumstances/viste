# concepts

## Signals

- Inspired by FSharp.Data.Adaptive
- Based on easily cloneable values
- Hold a value at any time
- Lazy evaluation
- Transaction-based?
- Listeners (enqueue in transaction)?

## Event streams

- Push-based stream with moving values
- Exactly one consumer

## Observables

- Modeled after Rx
- Eager, push-based
- Based on either References or easily cloneable values
- Used as foundation for lists, maps etc.