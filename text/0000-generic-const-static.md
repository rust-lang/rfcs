- Feature Name: generic_const_static
- Start Date: 2016-02-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow generic `const`s and `static`s with the obvious syntax and semantics.

# Motivation
[motivation]: #motivation

I was surprised to realize this wasn't already supported.

The most important benefit is the ability to generate a separate static allocation in the binary for each used set of type parameters.  The use case for scalar-typed generic `const`s somewhat overlaps with `const fn`s, but when a static address is needed, Rust currently provides no alternative or workaround.  (In particular, `const`s and `static`s inside generic `fn`s cannot refer to the outer `fn`'s type parameters, nor can associated `const`s inside traits refer to trait generic parameters or to `Self`.  Actually, you can technically do it with `asm!`...)

rustc already generates static allocations based on type parameters for specialized function code itself (obviously) and for vtables of specialized types.  Indeed, I ran into this when trying to implement a custom vtable of sorts, something like:

```rust
struct HashKeyData {
    size: usize,
    eq: fn(*const u8, *const u8) -> bool,
    hash: fn(*const u8) -> usize,
}

fn eq_wrapper<T: Eq>(a: *const u8, b: *const u8) -> bool {
    let (a, b): (&T, &T) = unsafe { transmute((a, b)) };
    a == b
}
fn hash_wrapper<T: HashToUsize>(a: *const u8) -> usize {
    let a: &T = unsafe { transmute(a) };
    a.hash()
}

static HKD<T: Eq + HashToUsize>: HashKeyData = HashKeyData {
    size: size_of::<T>(),
    eq: eq_wrapper<T>,
    hash: hash_wrapper<T>,
}
```

(This is slightly simplified, but the idea is to use unsafe code to replicate the efficient storage of a generic type, without the binary size overhead of real generic functions.  We could just store the `HashKeyData` inline in the hash table struct, but that would waste space, especially because more fields are needed than I've shown; better to use a static allocation.)

# Detailed design
[design]: #detailed-design

- The grammar for `static` and `const` (including `static mut` and associated `const`s) is changed to accept an optional generic parameter list after the identifier, and `where` clause before the `=` (or `;` in the case of associated `const`s).  Another example:

```rust
// Hypothetical trait that lets you get a `Default` impl, a const version, and
// a static version in one shot (would have to be in libcore due to coherence).
// If you often need to pass references to the default value, using the static
// is faster than creating a new copy on the stack each time.
trait ConstDefault: Sized {
    const DEFAULT: Self;
}
impl<T: ConstDefault> Default for T {
    fn default() -> Self { Self::DEFAULT }
}
static DEFAULT<T>: T where T: ConstDefault = T::DEFAULT;
```

- Note that ending a where clause with an `=` token has precedent in `type`, so this could not create a new ambiguity in the presence of hypothetical future grammar.

- Both the type and the initializer can depend on generic parameters.  As usual, they are resolved and typechecked before generic substitution based on the constraints present.

- Generic `static`s are monomorphized to a separate static allocation per specialization.  (They are guaranteed to have different addresses iff separately written static declarations are so guaranteed; I'm not sure if Rust plans to support coalescing identical truly-immutable statics.)

- As with generic `fn`s, generic `static`s should not be located in an `extern` block or marked `#[no_mangle]`.  (For some reason, in the `fn` case, the former is currently a hard error while the latter is only a warn-by-default lint; this RFC leaves it up to the implementers whether to preserve this behavior for `static`s.)


# Drawbacks
[drawbacks]: #drawbacks

- Increases the number of language features, but makes the language more regular in return.

- Adds a small wrinkle to hypothetical source-to-source translation of Rust to JITted languages by type erasure; many already exist, though.

# Alternatives
[alternatives]: #alternatives

- Support only one of `const` and `static`.  Either would be able to satisfy the motivation, but the features are similar enough that this would almost certainly confuse more than it helped.

- As previously mentioned, supporting references to outer generic parameters inside inner items could be seen as an alternative, as it enables workarounds.  While I believe this should be done for other reasons, there doesn't seem to be much reason to require a workaround when the direct solution is simple enough.

# Unresolved questions
[unresolved]: #unresolved-questions

- ?

