#![no_std]

/*!
`occupied` provides compile-time guaranteed ways to interact with inserting
and removing items into [`Option`]. This simplifies more complicated access
patterns, when you're interacting with a handful of options and don't want to
[`.take()`][Option::take] anything out of them until they've all been verified
somehow.

# Example

Suppose you had an array of options, and wanted to unwrap them all, but only
if they're all `Some`, and leave them all untouched otherwise. If you own the
array, this is easy to do with a `match`, but if not, you have to either:

1. Manually check `.is_some()` on all of them, and then `.unwrap()` them only
   after they've all been checked, or
2. `.take()` them one-by-one, and take care to *restore* each option to its
   original state after.

`occupied` provides a way to preserve the `is_some()` check within the type
system, making it checked at compile-time, without actually mutating the option.

```
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
*/

use core::hint::unreachable_unchecked;

/// Hide implementation details in a submodule, to contain the sites where
/// `Occupied.option` and `Vacant.option` can be accessed directly (because
/// that can be done without `unsafe`). We'd rather force the use of `unsafe{}`
/// to call the relevant methods.
mod internals {
    mod occupied {
        /**
        A reference to an [`Option`] that is statically guaranteed to be occupied,
        meaning we can [`.take()`][Occupied::take] the object out unconditionally,
        and infallibly, leaving a [`None`] in its place.
        */
        #[derive(Debug)]
        pub struct Occupied<'a, T> {
            option: &'a mut Option<T>,
        }

        impl<'a, T> Occupied<'a, T> {
            /**
            Create a new [`Occupied`], referencing an [`Option`] that is definitely
            [`Some`].

            # Safety

            The `option` parameter MUST be [`Some`].
            */
            #[inline(always)]
            #[must_use]
            pub const unsafe fn new_unchecked(option: &'a mut Option<T>) -> Self {
                debug_assert!(option.is_some());
                Self { option }
            }

            /**
            Get an immutable reference to the data in the referenced option.

            # Example

            ```
            use occupied::OptionExt as _;

            let mut opt = Some("hello");
            let occupied = opt.peek_some().unwrap();

            assert_eq!(*occupied.get(), "hello");
            ```
            */
            #[inline(always)]
            #[must_use]
            pub const fn get(&self) -> &T {
                debug_assert!(self.option.is_some());
                unsafe { self.option.as_ref().unwrap_unchecked() }
            }

            /**
            Get an mutable reference to the data in the referenced option.

            # Example

            ```
            use occupied::OptionExt as _;

            let mut opt = Some("hello");
            let mut occupied = opt.peek_some().unwrap();

            *occupied.get_mut() = "goodbye";

            assert_eq!(opt, Some("goodbye"));
            ```
            */
            #[inline(always)]
            #[must_use]
            pub const fn get_mut(&mut self) -> &mut T {
                debug_assert!(self.option.is_some());
                unsafe { self.option.as_mut().unwrap_unchecked() }
            }

            /**
            Get a mutable reference to the underlying [`Option`]. This destroys
            `self`, because we lose the guarantee that the option is occupied.

            # Example

            ```
            use occupied::OptionExt as _;

            let mut opt = Some("hello");
            let mut occupied = opt.peek_some().unwrap();

            let reference = occupied.into_inner();
            *reference = None;

            assert_eq!(opt, None)
            ```
            */
            #[inline(always)]
            #[must_use]
            pub const fn into_inner(self) -> &'a mut Option<T> {
                self.option
            }
        }
    }

    mod vacant {
        /**
        A reference to an [`Option`] that is statically guaranteed to be vacant.
        This type is fairly niche, but it allows *slightly* more efficient inserts
        into the referenced option.
        */
        #[derive(Debug)]
        pub struct Vacant<'a, T> {
            option: &'a mut Option<T>,
        }

        impl<'a, T> Vacant<'a, T> {
            /**
            Create a new [`Vacant`], referencing an [`Option`] which is guaranteed
            to be [`None`].

            # Safety

            The referenced option *must* be [`None`].
             */
            #[inline(always)]
            #[must_use]
            pub const unsafe fn new_unchecked(option: &'a mut Option<T>) -> Self {
                debug_assert!(option.is_none());
                Self { option }
            }

            /**
            Get a mutable reference to the underlying [`Option`]. This destroys
            `self`, because we lose the guarantee that the option is vacant.

            # Example

            ```
            use occupied::OptionExt as _;

            let mut opt = Some("hello");
            let mut occupied = opt.peek_some().unwrap();

            let (vacant, item) = occupied.extract();
            assert_eq!(item, "hello");

            let reference = vacant.into_inner();
            assert!(reference.is_none());
            *reference = Some("goodbye");

            assert_eq!(opt, Some("goodbye"));
            ```
             */
            #[inline(always)]
            #[must_use]
            pub const fn into_inner(self) -> &'a mut Option<T> {
                self.option
            }
        }
    }

    pub use occupied::Occupied;
    pub use vacant::Vacant;
}

pub use internals::{Occupied, Vacant};

impl<'a, T> Occupied<'a, T> {
    /**
    Try to create a new [`Occupied`] instance, referencing an [`Option`] that is
    definitely [`Some`]. Returns [`None`] if the option is [`None`].
     */
    #[inline(always)]
    #[must_use]
    pub const fn new(option: &'a mut Option<T>) -> Option<Self> {
        // Use `examine` to reduce the amount of unsafe and trust that inlining
        // will produce efficient code.
        match examine(option) {
            Entry::Occupied(occupied) => Some(occupied),
            Entry::Vacant(_) => None,
        }
    }

    /**
    Get a mutable reference to the underlying value with the original
    lifetime.
    */
    #[inline(always)]
    #[must_use]
    pub const fn into_mut(self) -> &'a mut T {
        let option = self.into_inner();
        debug_assert!(option.is_some());

        // Safety: the option in `Occupied` is guaranteed to be `Some`
        unsafe { option.as_mut().unwrap_unchecked() }
    }

    /**
    Remove the item from the [`Option`], leaving [`None`] in its place.

    # Example

    ```
    use occupied::OptionExt as _;

    let mut opt = Some("Hello");
    let occupied = opt.peek_some().unwrap();

    assert_eq!(occupied.take(), "Hello");
    assert_eq!(opt, None);
    ```
     */
    #[inline(always)]
    pub const fn take(self) -> T {
        // TODO: currently, const limitations mean that we can't define this
        // in terms of extract; the compiler insists that it be possible to
        // call the destructor of the (Vacant, T) tuple even if it's
        // destructured.
        let option = self.into_inner();
        debug_assert!(option.is_some());

        // Safety: option from `Occupied` is guaranteed to be `Some`.
        unsafe { option.take().unwrap_unchecked() }
    }

    /**
    Identical to [`.take()`][Self::take], except that it also returns a
    [`Vacant`] instance, allowing something to later be inserted into the
    guaranteed-to-be-`None` option. Usually you can just use
    [`.take()`][Self::take].
    */
    #[inline(always)]
    pub const fn extract(self) -> (Vacant<'a, T>, T) {
        let option = self.into_inner();
        debug_assert!(option.is_some());

        // Safety: option from an `Occupied` is guaranteed to be `Some`
        let item = unsafe { option.take().unwrap_unchecked() };

        // Safety: option is guaranteed to be `None` after `take`
        (unsafe { Vacant::new_unchecked(option) }, item)
    }
}

impl<T> AsRef<T> for Occupied<'_, T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> AsMut<T> for Occupied<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}

impl<'a, T> Vacant<'a, T> {
    /**
    Try to create a new [`Vacant`] instance, referencing an [`Option`] that is
    definitely [`None`]. Returns [`None`] if the option is [`Some`].
     */
    #[inline(always)]
    #[must_use]
    pub const fn new(option: &'a mut Option<T>) -> Option<Self> {
        match examine(option) {
            Entry::Vacant(vacant) => Some(vacant),
            Entry::Occupied(_) => None,
        }
    }

    /**
    Insert an item into the [`Vacant`] option, and return an [`Occupied`]
    reference to the inserted item. This will be *slightly* more efficient
    than [`Option::insert`], since [`Option::insert`] must check whether
    the option is currently [`Some`] and destruct if so.
    */
    #[inline(always)]
    pub const fn insert(self, item: T) -> Occupied<'a, T> {
        let option = self.into_inner();
        debug_assert!(option.is_none());

        // Use an unreachable branch to avoid the conditional, since we
        // know the option is `None`
        match *option {
            // Safety: an option from a `Vacant` is always `None`
            Some(_) => unsafe { unreachable_unchecked() },

            None => {
                // Safety: `option` is, of course, safe to write to, it's
                // a regular mutable reference. This will leak the contents
                // of `option`, but we know that's just a `None`, so there's
                // no problem there. We can't use a regular assignment here
                // because of `const` limitations.
                unsafe { core::ptr::write(option, Some(item)) };

                // Safety: `option` is now guaranteed to be `Some`, since
                // we just wrote to it.
                unsafe { Occupied::new_unchecked(option) }
            }
        }
    }
}

/**
Wrapper around a mutable reference to an option, containing information about
whether the option is vacant or occupied.
*/
#[derive(Debug)]
pub enum Entry<'a, T> {
    /// The option is occupied
    Occupied(Occupied<'a, T>),

    /// The option is vacant
    Vacant(Vacant<'a, T>),
}

impl<'a, T> Entry<'a, T> {
    /// Modify the item in the option, if any.
    #[inline]
    pub fn and_modify(mut self, f: impl FnOnce(&mut T)) -> Self {
        if let Entry::Occupied(ref mut occupied) = self {
            f(occupied.as_mut())
        }

        self
    }

    /**
    Insert an item into the option if it isn't already occupied, and then return
    an [`Occupied`] reference to the now-occupied option.
     */
    #[inline(always)]
    pub fn or_insert(self, default: T) -> Occupied<'a, T> {
        self.or_insert_with(|| default)
    }

    /**
    Insert an item into the option if it isn't already occupied, using a
    function to produce the item, then return an [`Occupied`] reference to the
    now-occupied option.
     */
    #[inline]
    pub fn or_insert_with(self, default: impl FnOnce() -> T) -> Occupied<'a, T> {
        match self {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(vacant) => vacant.insert(default()),
        }
    }

    /**
    Remove the item from this option, if any, and return both the item and
    a [`Vacant`] reference to the now-vacant option.

    Unless you're doing something fancy, you should really just call `.take()`
    on the original [`Option`].
     */
    #[inline]
    pub const fn remove(self) -> (Option<T>, Vacant<'a, T>) {
        // In the future we'd rather just call `occupied.extract()`, but
        // limitations in the compiler's ability to reason about tuple
        // destructors in a const context prevent this for now.
        let opt = match self {
            Self::Occupied(occupied) => occupied.into_inner(),
            Self::Vacant(vacant) => vacant.into_inner(),
        };

        let item = opt.take();

        // Safety: after `take`, the `option` is guaranteed to be `None`
        (item, unsafe { Vacant::new_unchecked(opt) })
    }

    /**
    Consume this [`Entry`] and return a mutable reference to the original
    option.
     */
    #[inline]
    pub const fn into_inner(self) -> &'a mut Option<T> {
        match self {
            Entry::Occupied(occupied) => occupied.into_inner(),
            Entry::Vacant(vacant) => vacant.into_inner(),
        }
    }
}

/**
Top level function to examine an option and return either an [`Occupied`]
reference, if it's occupied, or a [`Vacant`] reference, if it's vacant. Usually
you'll call [`.entry()`][OptionExt::entry] or [`.peek_some`][OptionExt::peek_some]
instead of this.
 */
#[inline]
pub const fn examine<T>(option: &mut Option<T>) -> Entry<'_, T> {
    match option {
        opt @ &mut Some(_) => Entry::Occupied(unsafe { Occupied::new_unchecked(opt) }),
        opt @ &mut None => Entry::Vacant(unsafe { Vacant::new_unchecked(opt) }),
    }
}

/**
Additional methods for [`Option`], granting access to [`Occupied`] and
[`Vacant`] references to its contents.
*/
pub trait OptionExt<T> {
    /**
    Try to get an [`Occupied`] reference to this option. Returns [`None`] if
    `self` is [`None`]; otherwise returns an [`Occupied`] which can be used
    to infallibly access the contained item or remove it from this [`Option`].
    */
    #[must_use]
    fn peek_some(&mut self) -> Option<Occupied<'_, T>>;

    /**
    Try to get an [`Vacant`] reference to this option. Returns [`None`] if
    `self` is [`Some`]; otherwise returns an [`Vacant`] which can be used
    to insert into the option.
    */
    #[must_use]
    fn peek_empty(&mut self) -> Option<Vacant<'_, T>>;

    /**
    Get an entry for this option, allowing in-place manipulation, insertion,
    or removal of the contained value.
    */
    #[must_use]
    fn entry(&mut self) -> Entry<'_, T>;

    /**
    Insert an item into this option, then return an [`Occupied`] reference to
    the now-occupied [`Option`]
     */
    fn emplace(&mut self, item: T) -> Occupied<'_, T>;

    /**
    Insert the `item` into the option, but only if the option is vacant. Either
    way, return an [`Occupied`] reference to the now-occupied [`Option`].
    */
    #[inline(always)]
    fn get_or_emplace(&mut self, item: T) -> Occupied<'_, T> {
        self.get_or_emplace_with(|| item)
    }

    /**
    Call `item` to get an item to insert into the option, but only if the
    option is vacant. Either way, return an [`Occupied`] reference to the
    now-occupied [`Option`].
    */
    fn get_or_emplace_with(&mut self, item: impl FnOnce() -> T) -> Occupied<'_, T>;
}

impl<T> OptionExt<T> for Option<T> {
    #[inline(always)]
    fn peek_some(&mut self) -> Option<Occupied<'_, T>> {
        Occupied::new(self)
    }

    #[inline(always)]
    fn peek_empty(&mut self) -> Option<Vacant<'_, T>> {
        Vacant::new(self)
    }

    #[inline(always)]
    fn entry(&mut self) -> Entry<'_, T> {
        examine(self)
    }

    #[inline(always)]
    fn emplace(&mut self, item: T) -> Occupied<'_, T> {
        *self = Some(item);

        // Safety: option is definitely Some at this point
        unsafe { Occupied::new_unchecked(self) }
    }

    #[inline]
    fn get_or_emplace_with(&mut self, item: impl FnOnce() -> T) -> Occupied<'_, T> {
        if self.is_none() {
            *self = Some(item());
        }

        // Safety: option is definitely Some at this point
        unsafe { Occupied::new_unchecked(self) }
    }
}
