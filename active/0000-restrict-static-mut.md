- Start Date: 2014-9-7
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

It is no longer allowed to directly modify the values in a `static mut` variable, even in unsafe code.
As a result of this, it is also perfectly safe to take immutable references to `static mut` variables.

# Motivation

There are numerous types that are very useful in static positions:

* `AtomicT`
* `Once`
* `StaticMutex`

All of these types are perfectly safe to use and mutate statically. However, due to the fact that
`static mut` variables can be modified without going through whatever type is stored, their use
currently requires unsafe code - while atomically modifying a variable is perfectly safe, avoiding
the atomicity by changing that actual `AtomicT` violates that safety.

# Detailed design

The built in ability for unsafe code to directly (without going through `UnsafeCell`) modify
`static mut` variables is removed. Taking a shared reference to a `static mut` variable no longer
requires unsafe code.

# Drawbacks

* This makes it harder for people to create unprotected static variables. This may not actually be a
  drawback, as global state is generally discouraged. Also, note that the old behavior may be imitated
  by storing an `UnsafeCell` directly in a static location.

# Alternatives

* #177. This allow creating shared references to `static mut` variables if the type contained implements
  `Share`. This has the same problem as the more drasting proposal described next.
* Allow safe code to take shared references to any `static mut` variables. In the absence of unsafe code,
  this is perfectly safe, because despite being `static mut`, the variables cannot be modified. However,
  it maked verifying the correctness of unsafe code much harder. When mutating a `static mut` variable,
  the unsafe code must not only preserve all of Rust's invariants, but also make sure not to conflict with
  any *safe* code that could have a reference to the variable. Currently, a user of unsafe only has to check
  for conflicts with other unsafe code, but this property is broken when an `&` pointer aliases a `*mut`
  pointer.

# Unresolved questions

* Does this make unprotected static variables too unwieldy?
* Will it be confusing to new users that something marked `mut` cannot be directly mutated?
