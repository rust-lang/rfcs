- Feature Name: leaky-boxes-dynamic-statics
- Start Date: (fill me in with today's date, YYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a safe static method `Box::leak` which leaks a Box<T> into a `&'static mut T`, preventing its destructor from being run, and allowing the data stored within it to be accessed for the remainder of the lifetime of the program. 

# Motivation

As `mem::forget` is safe, it is legal to not call the destructor of a type in safe rust.  Dynamically allocating static lifetime values is currently not possible in safe rust, but is safe.

There are times when you would want to safely leak a resource, yet maintain access to the resource (unlike `mem::forget` which leaks the resource but prevents references to it). There are potential programs which this could increase the ease of design, due to program-lifetime structs which would otherwise have to have lifetime propagation, but can instead be leaked to `'static` to simplify code, among other things.

# Detailed design

This method would be a static method on `Box` named `leak`.

Example Implementation:
```rust
impl<T : ?Sized> Box<T> {
    pub fn leak<'a>(b: Self) -> &'a mut T {
        unsafe { mem::transmute(b) }
    }
}
```

# Drawbacks

This adds a new function to the rust standard library which is not technically necessary, as the functionality of `Box::leak` can already be fairly easily implemented in rust, and is not necessarially a pattern which we want to encourage or enable.

# Alternatives

We could also implement this with a `'static` bound on the function, always producing a `&'static mut T`. This has the advantage of making the type signature more explicit, but loses some of the generality of the `leak` method to also support leaking types with arbitrary lifetime parameters.

Example implementation:
```rust
impl<T : ?Sized + 'static> Box<T> {
    pub fn leak(b: Self) -> &'static mut T {
        unsafe { mem::transmute(b) }
    }
}
```

If we don't implement `Box::leak`, the safe action of leaking a `Box<T>` into a `&'static mut T` will not be possible without using unsafe rust, although it will be very easy for crates to implement this functionality themselves.

# Unresolved questions

Should the method include a `'static` bound on it's type parameter `T`, and always produce a `&'static mut T`, rather than a `&'a mut T` for any lifetime `'a`?
