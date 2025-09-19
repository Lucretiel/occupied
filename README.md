# occupied

A simple utility library for transitioning enums into and out of occupied states

<!-- cargo-rdme start -->

`occupied` provides compile-time guaranteed ways to interact with inserting and removing items into [`Option`](https://doc.rust-lang.org/std/option/enum.Option.html). This simplifies more complicated access patterns, when you're interacting with a handful of options and don't want to [`.take()`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take) anything out of them until they've all been verified somehow.

## Example

Suppose you had an array of options, and wanted to unwrap them all, but only if they're all `Some`, and leave them all untouched otherwise. If you own the array, this is easy to do with a `match`, but if not, you have to either:

1. Manually check `.is_some()` on all of them, and then `.unwrap()` them only after they've all been checked, or
2. `.take()` them one-by-one, and take care to *restore* each option to its original state after.

`occupied` provides a way to preserve the `is_some()` check within the type system, making it checked at compile-time, without actually mutating the option.

```rust
use occupied::OptionExt as _;

fn try_unwrap_all<T>(options: &mut [Option<T>; 4]) -> Option<[T; 4]> {
    // (this code will be simpler when `array::try_map` is available)
    let [opt1, opt2, opt3, opt4] = options;

    // Create an of `Occupied` instances, guaranteeing that the underlying
    // options are `Some`
    let confirmed = [
        opt1.peek_some()?,
        opt2.peek_some()?,
        opt3.peek_some()?,
        opt4.peek_some()?,
    ];

    // `.take()` all of the values
    Some(confirmed.map(|item| item.take()))
}

let mut opts = [Some(1), Some(2), Some(3), None];

assert_eq!(try_unwrap_all(&mut opts), None);
assert_eq!(opts, [Some(1), Some(2), Some(3), None]);

opts[3] = Some(4);


assert_eq!(try_unwrap_all(&mut opts), Some([1, 2, 3, 4]));
assert_eq!(opts, [None, None, None, None]);
```

<!-- cargo-rdme end -->
