- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature could serve as the backbone for some pointer to field syntax, and even if no syntax is made, this feature serves as a safe generic way to talk about types and their fields.

# Motivation
[motivation]: #motivation

The motivation for this feature is to allow safe projection through smart pointers, for example `Pin<&mut T>` to `Pin<&mut Field>`. This is a much needed feature to make `Pin<P>` more usable in safe-contexts, without the need to use unsafe to map to a field. This also can allow projection through other smart pointers like `Rc<T>`, `Arc<T>`. This feature cannot be implemented as a library effectively because it depends on the layouts of types, so it requires integration with the Rust compiler until Rust gets a stable layout (which may never happen).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

First the core trait, type, and functions that need to be added

```rust
/// Contains metadata about how to get to the field from the parent using raw pointers
/// This is an opaque type that never needs to be stablized, and it only an implementation detail
struct MetaData {
    ...
}

/// The compiler should prevent user implementations for `Field`,
/// i.e. only the compiler is allowed to implement `Field`
/// This is to prevent people from creating fake "fields"
trait Field {
    /// The type that the field is a part of
    type Parent;
    /// The type of the field
    type Type;

    /// The metadata required to get to the field using raw pointers
    const META: MetaData;
}

trait Project<F: Field> {
    /// The projected version of Self
    type Projection;

    fn project(self, field: F) -> Self::Projection;
}

impl<T: ?Sized> *const T {
    unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> *const F::Type {
        // make the field pointer, this code is allowed to assume that
        // self points to a valid instance of T
        ... 
    }
}

impl<T: ?Sized> *mut T {
    unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> *mut F::Type {
        // make the field pointer, this code is allowed to assume that
        // self points to a valid instance of T
        ...
    }
}
```

Now we need some syntax to refer to the fields of types. Some ideas for the syntax are

* `Parent.field`
* `Parent::field` // bad as it conflicts with associated functions
* `Parent~field` // or any other sigil

We will call these field types, because they will desugar to a unit type that correctly implements `Field`, like so

```rust
struct Foo {
    bar: Bar
}

struct Foo.bar;

impl Field for Foo.bar { ... }
```

These are the core parts of this proposal. Every other part of this proposal can be postponed or dropped without affecting this feature's core principles.

Using these core parts we can build as a library projections through `Pin<&T>`, `Rc<_>` and more. We can then use this to safely project through smart pointers like so.

```rust
let foo   : Pin<Box<Foo>    = Box::pin(immovable);
let foo   : Pin<&mut Foo>   = foo.as_mut();
let field : Pin<&mut Field> = foo.project(Foo.field);
```
But to do safe pin projections we will need to introduce a marker trait.
```rust
/// The only people who can implement `PinProjectable` are the creator of the parent type
/// This allows people to opt-in to allowing their fields to be pin projectable.
/// The guarantee is that once you create `Pin<P<Parent>>`, all of the same guarantees that
/// apply to `Pin<P<Parent>>` also apply to `Pin<P<Field>>`
/// For all `Parent: Unpin`, these can be auto implemented for all of their fields.
unsafe trait PinProjectable: Field {}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The field types needs to interact with the privacy rules for fields. A field type has the same privacy as the field it is derived from. Anything else would be too restrictive or unsound.

As example of how to implement `Project`, here is the implementation for `&T`.

```rust
impl<'a, F: Field> Project<F> for &'a F::Parent where F::Type: 'a {
    type Projection = &'a F::Type;

    fn project(self, field: F) -> Self {
        unsafe {
            // This is safe because a reference is always valids
            let ptr: *const F::Type = (self as *const F::Parent).project_unchecked(field);

            &*ptr
        }
    }
}
```



This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
