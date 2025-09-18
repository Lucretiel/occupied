# occupied

A simple utility library for transitioning enums into and out of occupied states

<!-- cargo-rdme start -->

`occupied` provides compile-time guaranteed ways to interact with inserting and removing items into [`Option`](https://doc.rust-lang.org/std/option/enum.Option.html). This simplifies more complicated access patterns, when you're interacting with a handful of options and don't want to [`.take()`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take) anything out of them until they've all been verified somehow.

## Example

Suppose you had a tuple of options, and wanted to unwrap them all, but only
if they're all `Some`, and leave them all untouched otherwise. If you own the
tuple, this is easy to do with a `match`, but if not, you have to either:

1. Manually check `.is_some()` on all of them, and then `.unwrap()` them only
   after they've all been checked, or
2. `.take()` them one-by-one, and take care to *restore* each option to its
   original state after.

`occupied` provides a way to preserve the `is_some()` check within the type
system, making it checked at compile-time, without actually mutating the option.

```rust
use occupied::OptionExt as _;

fn try_unwrap_all<A, B, C, D>(
    options: &mut (Option<A>, Option<B>, Option<C>, Option<D>)
) -> Option<(A, B, C, D)> {
    // Create a tuple of `Occupied` instances, guaranteeing that the underlying
    // options are `Some`
    let confirmed = (
        options.0.peek_some()?,
        options.1.peek_some()?,
        options.2.peek_some()?,
        options.3.peek_some()?,
    );

    // `.take()` all of the values
    Some((
        confirmed.0.take(),
        confirmed.1.take(),
        confirmed.2.take(),
        confirmed.3.take(),
    ))
}

let mut opts = (Some(1), Some(2), Some(3), None);

assert_eq!(try_unwrap_all(&mut opts), None);
assert_eq!(opts, (Some(1), Some(2), Some(3), None));

opts.3 = Some(4);


assert_eq!(try_unwrap_all(&mut opts), Some((1, 2, 3, 4)));
assert_eq!(opts, (None, None, None, None));
```

<!-- cargo-rdme end -->
