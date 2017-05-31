- Start Date: (fill me in with today's date, YYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Change Rust's variance inference rules to make them less error-prone.

# Motivation

Rust currently employs automatic variance inference to decide whether
a type parameter is covariant, contravariant, or invariant. The
current inference scheme is as general as possible but sometimes leads
to surprising behavior, particularly around type or lifetime
parameters that are not used.

For example, consider the following struct definition:

    struct Wrap<T>;
    
Here, the type parameter `T` is not used at all. The inference
algorithm will infer that this is *bivariant*, meaning that no matter
what value we supply for `T`, accesses to the fields of `Wrap` will
continue to be safe. In other words, if I have `&Wrap<int>`, that is
interchangable with `&Wrap<uint>`. The algorithm is not wrong. Since
`Wrap` has no fields, there is nothing you can do with this `T` that
is incorrect per se. But the result is certainly surprising.

The current algorithm is particularly broken when people employ unsafe
code that may encode uses for types that are not immediately evident.
For example, one idiom is to attach a lifetime to a struct that
contains unsafe pointers, to prevent that struct from being used
outside of a certain scope. For example, at some point the iterator
for a vector was defined as follows:

    struct Elements<'a, T> {
        start: *T, end: *T
    }
    
Here of course, the lifetime `'a` is not used, and hence the inference
algorithm again infers bivariance. This effectively means that the
lifetime parameter `'a` is completely ignored, which is certainly not
what was intended.

The current way to control the inference algorithm is to employ marker
types like `CovariantType` or `ContravariantLifetime`. These are
maximally flexible but also dangerous as defaults -- if you don't know
that you need to use them, you will get very lax typing, much laxer
than you might expect.

I'd like to remove the need for marker types altogether. Currently
they are used to opt out of the builtin kinds, but we are moving to an
opt-in system which makes those markers unnecessary. The remaining
place marker types are used is for specifying variance, but with this
proposal they will not be needed there either.

# Detailed design

The proposal has three parts:

1. Change the default behavior of the inference algorithm as follows:
   If a type or lifetime parameter is not used within the body of a
   type, we default to covariance for types and contravariance for
   lifetimes.
2. Treat `Unsafe<T>` as invariant with respect to `T`. This is not
   strictly necessary (the current behavior can be simulated,
   described below), but it's more convenient for users.
3. Remove the marker types for variance inference.

Let's address each part in turn.

## Change 1. Adjust how inference algorithm treats unused parameters.

This change chooses as defaults what I believe most people would expect.
More specifically, if you write a struct with an unused type parameter `T`:

    struct Foo<T> { } // Note: T is not used
    
That is equivalent (from the variance algorithm's point of view) to
a struct that contains an instance of `T`:

    struct Foo<T> { field: T } // Foo<T> above equivalent to this
    
Similarly, if you write a struct with an unused lifetime parameter `'a`:

    struct Bar<'a> { } // Note: 'a is not used
    
That is equivalent to a struct containing a reference of lifetime `'a`:

    struct Bar<'a> { f: &'a () } // Bar<'a> above equivalent to this
    
This is what I intuitively expect when I see `Foo<T>` or `Bar<'a>`.

## Change 2. Treat `Unsafe<T>` as invariant in the inference analysis.

We've already decided that interior mutability which does not build on
`Unsafe<T>` is undefined behavior (this is needed to prevent segfaults
and so forth from static constants). Therefore, since `Unsafe<T>` is
built into the language rules, it just makes sense for `Unsafe<T>` to
be known to the variance inference as well. The inference algorithm
can treat `T` as invariant, which means there will be no need for a
marker type here.

This is more convenient for users since an `Unsafe` static constant
can be created with having to write out any markers:

```
// Before
static UNSAFE_ZERO: Unsafe<uint> = Unsafe { value: 0,
                                            marker: marker::InvariantType };
                                            
// After:                                            
static UNSAFE_ZERO: Unsafe<uint> = Unsafe { value: 0 };
```

Note that we could not make this change, but it would require keeping
markers or something equivalent to markers, as described in the next
section.

## Change 3. Remove marker types.

If we change the algorithm as described above, then I think there is
no longer any need for marker types.

One reason for this is that their effect can be completely simulated:

```
struct CovariantType<T>;
struct ContravariantType<T> { m: CovariantType<fn(T)> }
struct InvariantType<T> { m: CovariantType<fn(T) -> T> }
struct CovariantLifetime<'a> { m: CovariantType<fn(&'a int)> }
struct ContravariantLifetime<'a>;
struct InvariantLifetime<'a> { m: CovariantType<fn(&'a int) -> &'a int> }
```

But the real reason for this change is not precisely that the current
markers can be simulated; it's more that I don't see the need for
them. Previously, there were two known situations where inference
was insufficient and markers were needed:

1. Unused lifetimes not capturing the expected constraint. Addressed by
   Change 1 above.
2. Interior mutability (`Cell<T>`, `RefCell<T>`), which should be invariant
   with respect to `T`. This is addressed by Change 2.

# Alternatives

There are many possible alternatives:

**Keep the current system, with the attendant risks.** I think we'll
see broken code this way, where users are not getting the type system
guarantees they think they are getting.

**Default to invariance for unused type parameters, not covariance.**
This would mean that, e.g., `Foo<&'static int>` is not a subtype of
`Foo<&'a int>`. Probably not much difference in practice, but I think
it's slightly more intuitive to follow the rule that unused type
parameters act *as if* they appeared the struct had a member of that
type.

**Make it an error to have an unused type or lifetime parameters,
except for in marker types.** This is the most explicit system, but
also requires that we keep marker types (which are undeniably awkward)
and requires the most direct interaction with variance annotations.

# Unresolved questions

None.
