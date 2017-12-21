- Feature Name: Limits trait for the rust types
- Start Date: 2017-12-20
- RFC PR: 
- Rust Issue: 

# Summary
This is an RFC to add a universal trait for the type limits.

# Motivation
The motivation is quite simple: [make an ability to accept template types with with limits by requiring a trait](https://stackoverflow.com/questions/47904954/rust-finding-the-maximum-allowable-value-for-generic-data-type-t) 
so that it simplifies and generalizes the code. Another motivation is that we have all that `max_value()` and `min_value()` implemented as
usual methods of a type implementation, generalizing this to a trait makes the code simplier and avoids duplicating code. Also, looking at
C++ template of `std::numeric_limits` tells us we must have this thing too because it is easier to use.

# Detailed design
The design is quite simple: put everything related to the limits what can be generalized into a separate trait in the standard library:

```rust
trait Limits<T> {
    fn min_value() -> T;
    fn max_value() -> T;
}

#[derive(Debug, Copy, Clone)]
struct A {
    field: u64,
}
impl Limits<A> for A {
    fn min_value() -> A {
        A {
            field: 0u64,
        }
    }

    fn max_value() -> A {
        A {
            field: 5u64,
        }
    }
}
impl Limits<u32> for u32 {
    fn min_value() -> u32 {
        0
    }
    fn max_value() -> u32 {
        10u32
    }
}

fn get_limits<T: Limits<T> + std::fmt::Debug>(_t: T) {
    println!("Minimum value: {:?}", T::min_value());
    println!("Maximum value: {:?}", T::max_value());
}

fn main() {
    let a = A { field: 6u64 };
    let num = 10u32;
    get_limits(a);
    get_limits(num);
}

```

Here we have a generalized function `get_limits` which accepts its argument with requirement for trait `Limits` implementation. As long
as a type implements this trait, this function will succeed and will produce expected results. It's worth mentioning that a type can implement
different limits type, not only for itself: `struct A` can have both `Limits<A>` and `Limits<u32>` implementations: we may simply add another
implementation and use it in our generalized function appropriately:

```rust
impl Limits<u32> for A {
    fn min_value() -> u32 {
        0u32
    }

    fn max_value() -> u32 {
        5u32
    }
}


/// Will be called only if a type implements Limits with u32 value type.
fn get_limits<T: Limits<u32> + std::fmt::Debug>(_t: T) {
    println!("Minimum value: {:?}", T::min_value());
    println!("Maximum value: {:?}", T::max_value());
}
```

Another option is to use the [`Bounded`](http://rust-num.github.io/num/num/trait.Bounded.html) trait of `num` crate:

```rust
trait Limits {
    fn min_value() -> Self;
    fn max_value() -> Self;
}
```

This will reduce the ambiguity of types and in most cases will be enough too.

# How We Teach This
I think the `Limits` name of a trait is appropriate:
- `numeric_limits` is incorrect since the trait may be implemented for any type and it may be not numerical.
- `Bounds` does not seem to be appropriate (personally for me).

This feature does not involve anything into the language itself, but adds a trait into the standard library. All the primitive types
and anything else what has `min_value()` and `max_value()` methods must implement this trait. Removing the type method is not required
(why does it work with having both a trait implementation and a type method - I don't know).

This feature can be introduced as `Limits` trait for the generalized contexts.

# Drawbacks
I don't know why we should not do this.

# Alternatives
Another option is to simply add macros for this:

```rust
macro_rules! max_value {
    ($type: ident) => {
        $type::max_value()
    }
}

macro_rules! min_value {
    ($type: ident) => {
        $type::min_value()
    }
}
```

This helps in generalizing the code too, but not in a way that the trait does.

# Unresolved questions
The trait design is arguable and is ready to accept any critic, namely what is better: generic trait or a simple one.
