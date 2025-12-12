- Feature Name: `const_fn_in_trait`
- Start Date: 2023-09-15
- RFC PR: [rust-lang/rfcs#3490](https://github.com/rust-lang/rfcs/pull/3490)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This feature allows marking functions in traits as `const`. Users of the trait
will be able to use these functions in const contexts.

# Motivation

[motivation]: #motivation

Currently, there is no implementation for high-level generic programming in
const contexts. Typically traits provide generic interfaces in Rust, but they do
not support `const` methods.

This RFC proposes allowing functions in a trait to be marked `const`, meaning
they can be called from a const context like other const functions. Use cases
include:

- Reducing code duplication in const contexts
- Providing a constructor for generic static object, e.g. C-style plugin
  registration
- Subtraits that want to provide defaults based on supertraits
- Compile-time checking of object properties
- Logically mononolithic traits that need to be split in order to support const
  actions

Workarounds typically involve a combination of wrapper functions, macros, and
associated consts. This RFC will eliminate the need for such workarounds.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Functions within a trait can be marked const:

```rust
trait GlobalState {
    /// Create a state that will be held in a global static
    const fn build(base_value: u32) -> State;
}
```

This indicates that all implementers must provide const functions:

```rust
struct Bar;

impl GlobalState for Bar {
    const fn build(base_value: u32) -> State { /* ... */ }
}
```

And then the function can be called in const contexts, including as generic
calls within other const functions:

```rust
/// Add a named item to our global state register
const fn register_state<T: GlobalState>(name: &'static str) {
    // ...
    STATES[n] = (name, T::build(n as u32))
}

/// Or, use with a single item
static DEFAULT_STATE: State = MyFavoriteStruct::build(42);
```

The rules for what is allowed are the same as for other `const` functions. At
runtime there is no difference with non-`const` trait functions.

`const` and non-const functions can coexist within the same trait, i.e. one
function being const does not mean all functions must be const.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

Trait functions will need to track an additional attribute that indicates
`const`ness. All implementers must match the same `const`ness of the original
trait's function definitions.

After monomorphization, these functions will be evaluated the same way as
standard `const` functions when needed at CTFE. This additional metadata can be
stripped in all other cases and the function will act the same as a non-const
function.

In short, this:

```rust
trait Foo {
    const fn foo(&self) -> u32;
}

impl Foo for Bar { /* ... */ }
```

Should be effectively treated the same as this at compile time:

```rust
const fn bar_as_foo_foo(bar: &Bar) -> u32 { /* ... */ }
```

And as this at runtime:

```rust
trait Foo {
    /* non-const */ fn foo(&self) -> u32;
}
```
## Relationship with the Keyword Generics Initiative

The [keyword generics initiative] or effects initiative proposes a way to have
optional `const`ness and `async`ness on trait functions, roughly:


```rust
// Note that syntax has changed a few times, including `~const`
const<C> trait SometimesConstFoo {
  const <C> fn sometimes_const_foo(&self) -> u32
}
```

This RFC aims to extract an extremely minimal subset of effects in order to make
it available sooner, similar to [`async-fn-in-trait`]. Part of why this proposed
design is so minimal is to avoid possible conflicts with effects.

# Drawbacks

[drawbacks]: #drawbacks

This feature requires tracking more information related to trait definition and
usage, but this should be a relatively maintenance burden.

There is also potential user confusion due to possible more content in a `trait`
block, as well as the question "does this need to be const". However, teaching
about `const`ness that applies to standard functions will generally apply here.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

There is currently no way to create generic functions that can be used in const
contexts. Workarounds exist but they are typically awkward, using a combination
of wrapper functions and macros to produce similar results.

Adding this feature approaches the goal of "Rust working like one would expect
it to work". That is, functions in traits generally act similar to functions
outside of traits, and this feature serves to bridge one of the final gaps

This feature is small so there are no real alternatives outside of the status
quo workarounds. The [keyword generics initiative] (effects initiative) will be
able to provide similar functionality; however, that is a much more in-depth
solution using parameterized optional constness. This feature should not
conflict with anything introduced as part of that proposal.

## Usage with `&dyn`

To be on the safe side, calling const functions from compile-time trait objects
is not allowed. This would look something like the below, using the same `trait
Foo` as above:

```rust
// Signature is OK and will compile (as it currently does)
const fn comptime_dyn_foo(object: &dyn Foo) {
    // This call is not allowed for the time being
    object.foo();
}
```

In theory, this behavior should be possible to support. However, it requires
more in-depth design than just tracking `const`ness through monomorphization; in
order to keep RFC suface area minimal, this is considered a future possibility.

Of course, the `const` function can still be called as a standard runtime
function:

```rust
fn runtime_dyn_foo(object: &dyn Foo) {
    object.foo();
}
```


# Prior art

[prior-art]: #prior-art

The [const function RFC](https://rust-lang.github.io/rfcs/0911-const-fn.html)
provides a reference for why `const` functions in Rust are generally useful.

[`async-fn-in-trait`] is a similar case of making standard function effects
available within traits. In this case, `async` comes with a lot more nuance than
`const`, so implementation effort for this RFC will most likely be much lower.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities

[future-possibilities]: #future-possibilities

- As part of the work of the accepted but not yet implemented [`refined-impls`]
  RFC, it may be possible to mark a function `const` in an implementation even
  if the trait signature does not indicate `const`.
- Calling `const` functions through `&dyn` could be added, as in [Usage with
  &dyn](#usage-with-dyn). This is likely blocked on having effects as function
  parameters, that is:
  ```rust
  // This works
  type F = fn() -> u32;
  type U = unsafe fn() -> u32;

  // But this does not yet work
  // type C = const fn() -> u32;
  // type A = async fn() -> u32;
  ```
- The [keyword generics initiative] will add much more fine tuned control
  than the basic mechanics in this RFC, allowing for optional const bounds
  in a parametric way.

[keyword generics initiative]: https://github.com/rust-lang/keyword-generics-initiative
[`async-fn-in-trait`]: https://rust-lang.github.io/rfcs/3185-static-async-fn-in-trait.html
[`refined-impls`]: https://rust-lang.github.io/rfcs/3245-refined-impls.html
