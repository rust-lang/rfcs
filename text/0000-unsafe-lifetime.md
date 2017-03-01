- Feature Name: `unsafe_lifetime`
- Start Date: 2017-02-21
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Add a new special lifetime, `'unsafe`, that implicitly satisfies any constraint, but may only be instantiated within an unsafe context. This lifetime is used in situations where the true lifetime of a value is otherwise inexpressible, and additionally serves as a warning to readers that handling of the value requires special care. As a result, unsafe code is no longer ever required to use misleading "false" or redundant lifetimes, instead clearly stating that the invariants are maintained by custom logic instead of borrowck.

# Motivation
[motivation]: #motivation

In the course of writing unsafe code, the need can arise to store a value of inexpressible lifetime. One common impetus for this pattern is when an unsafe data structure must be internally self-referential. This pattern may even be observed in the standard library:

core/cell.rs:871:

```rust
#[stable(feature = "rust1", since = "1.0.0")]
pub struct Ref<'b, T: ?Sized + 'b> {
    value: &'b T,
    borrow: BorrowRef<'b>,
}
```

Here, the lifetime of `value` is bounded not just by the specified scope, `'b`, but also by the continued existence of the `borrow` value beside it. Dropping this value renders the reference invalid, but that fact is unclear from a glance at the code.

 In cases such as this, the value is insulated from the outside world and exposed through a safe interface. This is all well and good, but we must still hold the value internally somehow, and if it demands lifetime parameters, we must provide them, even if they are false, hence the `&'b T` as above. In other cases where an explicit lifetime parameter is not available, it might also suffice to simply use `'static` as a surrogate lifetime and coerce the value as appropriate. This approach is workable, but has two major problems.

* First, the intent is unclear and must be expressed as a comment. The claim that the value is valid for `'b` or `'static` or what-have-you is a lie and one must keep in mind what the actual true lifetime is. Later modifications of the code, particularly by those other than the original author, may overlook this detail and accidentally misuse the value.

* Second, even `'static` is not flexible enough in all cases. While in many cases it will work, `'static` has meaningful semantic implications of its own and cannot act as a stand-in for any possible lifetime. Case in point:

```rust
struct SelfRefStruct<T> {
    owner: RefCell<T>,
    borrower: Ref<'static, T>, // Problem, T is not 'static, this type can't exist
}
```

Here, `'static` fails us in that it violates the constraints required by the declaration of `Ref`. Since this lifetime is false anyway, this fact shouldn't matter to us, but we currently have no way of telling the compiler that we're assuming the responsibility of maintaining the proper lifetime semantics. In such cases, we must resort to a workaround:

```rust
struct SelfRefStruct<'redundant, T: 'redundant> {
    owner: RefCell<T>,
    borrower: Ref<'redundant, T>, // Compiler is happy again
}
```

Now the compiler is satisfied, but we've paid a price. Or rather, we passed the cost along to the consumer of our API and _they_ must pay the price of supplying this redundant lifetime that has no actual meaning other than to satisfy the compiler. In some cases inference will eliminate it for us, but that won't always work. If we want to put `SelfRefStruct` in a struct of our own, then that struct will also be infected with the redundant lifetime, and so on. `'unsafe`, however, provides us a means to insulate the user from this implementation detail:

```rust
struct SelfRefStruct<T> {
    owner: RefCell<T>,
    borrower: Ref<'unsafe, T>, // Force the compiler to accept this type without an additional parameter.
}
```

# Detailed design
[design]: #detailed-design

## Semantics
Fortunately, the language already has precedent for similar semantics in some contexts, via HRTB or unbounded lifetimes. A lifetime declared within an HRTB bound implicitly satisfies the constraints that are demanded of it within. Similarly, an unbounded lifetime as described in the nomicon can be coerced into any type signature, including cases where even `'static` is inadequate. This RFC proposes merely to allow such a lifetime to be nameable and used within more contexts, such as struct fields.

As raw pointers provide an escape-hatch for the borrow checker to references, so `'unsafe` does to arbitrary types parameterized by lifetime. However, there is an important distinction. Raw pointers may safely exist and be manipulated, but they are unsafe to dereference at the site of use. `'unsafe` has inverted semantics, in that it is unsafe to instantiate a value of such a lifetime in the first place, but after it exists, it may be used as if it were safe. This property is important to maintain lifetime parametricity.

## Inference
`'unsafe` should never be inferred as a lifetime without being explicitly stated. Even an unbounded lifetime will not infer to be `'unsafe` unless the target value has been explicitly declared to require unsafe lifetime. This will preserve current behavior in all existing code, and by extension backward compatibility.

## Coercion
As an unbounded lifetime, `'unsafe` will readily decay into a concrete lifetime. It can be considered to be variance-compatible with all other lifetimes. Calling a method on an object of unsafe lifetime can, like any other lifetime, result in a reborrow of a shorter, concrete lifetime for the scope of the call. In this way, `'unsafe` may be consumed by any API without special consideration, since it can be coerced into something meeting the constraints required of that API.

If `'unsafe` appears multiple times within the type signature of a value, each instance may be coerced independently; multiple instances of `'unsafe` are not implied to represent the same concrete lifetime after coercion. In this way, `'unsafe` can be thought of as a wildcard.

## Drop
When an owned value of unsafe lifetime reaches the end of a scope, it will be assumed to be valid at that point and dropped as any other owned value would be.

## Optimization
Since an object of unsafe lifetime cannot be statically known to be valid at any particular time, the compiler should not assume validity merely based on accessibility of the value. Explicit use of such a value is a promise to the compiler that it is valid at that point, but such use should not be reordered relative to other linear code with observable semantics. However, coercing `'unsafe` into a concrete lifetime constitutes a promise to the compiler that the reborrow is safe and valid for the duration of the lifetime into which it was coerced, so full optimization may take place as per usual within such scopes.

## Safety
Finally, given all of the above, what manipulations, exactly, of a value of unsafe lifetime require unsafe scope?

Since our goal is to allow for encapsulating away the details of an internally unsafe value, merely moving such an object is considered safe. This includes objects that directly expose `'unsafe` in their type signature (e.g. `Ref<'unsafe, i32>`.

An object of unsafe lifetime may be coerced into an object of any other lifetime, and doing so shall be considered safe, as the assertion of possible unsafety will already have been made upon the creation of the value within an unsafe context.

Any expression in which `'unsafe` is used _literally_ as a lifetime argument, whether by inference or explicitly, is unsafe. Since `'unsafe` can only be inferred as a lifetime when explicitly required by the destination value, most expressions on such a value will instead infer to a concrete lifetime and will therefore be considered safe.

The above condition specifies "literally" in that it is possible, say, for a function parameterized over lifetimes to receive `'unsafe` as its argument, but the internal implementation is unaware of this and can only refer to the argument by its parameter name (e.g. `'a`). In such cases, the standard borrowck logic ensures internal consistency, so any invocations on such a lifetime are considered safe, even if `'a` has, in this particular instance, been bound with `'unsafe`. This is still sound overall since the only way a lifetime parameter can be bound as `'unsafe` is for it to be explicitly named so at some point, and doing so requires unsafe permissions to begin the chain.

Finally, it is considered legal and safe to coerce a value of unbounded lifetime into a value of unsafe lifetime, since doing so implies no additional unsafety.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

If a value has taken `'unsafe` as one of its lifetime parameters, it is said to have "unsafe lifetime" and requires special care and attention in its use. Such values should be isolated as tightly as possible behind a safe interface.

This concept is relatively niche and is not something that most rust users will need to be concerned with or use. As a result, its use can be described only in the more advanced materials such as the nomicon, likely alongside the HRTB and unbounded lifetimes sections.

# Drawbacks
[drawbacks]: #drawbacks

* Increased implementation complexity in borrowck. Perhaps existing similar semantics can be leveraged to mitigate this complexity somewhat, but it exists nonetheless.
* Increases the number of moving parts in an unsafe block. The list of things that unsafe allows is relatively short already, but adding another item to that list does make it more complex to describe the full implications of unsafe.
*  `'unsafe` does not indicate even a vaguely appropriate scope for the value, leaving it to be described in comments. However, since this is intended for use in cases where no other lifetime is appropriate, any concrete lifetime would be a lie and thus perhaps even more misleading than making no claims about the useful scope at all, but this is debatable.

# Alternatives
[alternatives]: #alternatives

* Implement self-referential structs. Many, if perhaps not all, use cases for this could perhaps be also expressed with self-referential structs. However, that is a far more complex feature. In the meantime, the presence of this feature enables the implementation of self-referential structs at the library level today, if not with ideal ergonomics.
* Use `for<'a> Ref<'a, T>` syntax instead of `Ref<'unsafe, T>`. This is technically a more accurate depiction of the nature of the type, but is arguably misleading in that it does not immediately evoke notions of unsafety.
* Do nothing. As described above, `'static` or other concrete lifetimes can sometimes be used as stand-ins, or in the worst case additional, redundant lifetime parameters. While it is possible to work around the limitations, the implementation details are leaked to the API consumer and cannot be fully encapsulated.

# Unresolved questions
[unresolved]: #unresolved-questions

* Exactly how complex would the implementation of this feature be? Is it just giving a name to a concept that already exists internally, or does it have farther reaching implications?
* In what contexts would `'unsafe` be permitted exactly. Struct fields are the primary concern, but is it also acceptable to allow it in function parameters? if so, is the function itself unsafe?
