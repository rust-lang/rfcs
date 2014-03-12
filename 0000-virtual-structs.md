- Start Date: 2014-03-17
- RFC PR #: 5
- Rust Issue #: 

# Summary

Virtual Structs

Allow inheritance between structs and virtual methods on struct objects. This
would support data structures such as the DOM which need to be efficient both in
speed (particularly non-virtual field access) and space (thin pointers).

# Motivation

Supporting efficient, heterogeneous data structures such as the DOM. Precisely
we need a form of code sharing which satisfies the following constraints:

* Cheap field access from internal methods;
* Cheap dynamic dispatch of methods;
* Cheap downcasting;
* Thin pointers;
* Sharing of fields and methods between definitions;
* Safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.

Example (in pseudo-code):

```
class Element {
    Element parent, left-sibling, right-sibling;
    Element[] children;

    foo();

    template() {
        x = foo();
        ...
    }
}

class Element1 : Element {
    Data some-data;

    template() {
        return some-data;
    }
}

final class Element2 : Element {
    ...
}
```

# Detailed design

Structs may be given the `virtual` keyword. That allows them to be extended by
other structs. Fields in super-structs are visible and usable (subject to the
usual module-level privacy checks) by the extending structs. Methods defined in
an impl for a super-struct exist for the sub-struct. We will not allow field
shadowing. Methods may be marked as `virtual` which allows them to be overriden
in the sub-struct's impl. Overriding methods must be marked `override`. It is an
error for a method to override without the annotation, or for an annotated
method not to override a super-struct method. (Methods marked `override` but not
`virtual` may not be overriden). Virtual methods may be given without a body,
these are pure virtual in the C++ terminology. Structs with pure virtual methods
(either explicit or inherited) may not be instantiated. Non virtual methods will
be statically dispatched as they are currently.

Pointer to a sub-struct is a subtype of a pointer to a super-struct.

Pointers to virtual structs may be downcast to the same pointer to a sub-struct.
This will be checked at runtime to ensure safety. Question: syntax for
downcasts? Should we use `as`? We should be able to check the runtime type to
ensure casting won't fail and/or be able to recover from a bad cast (see the
example below for some ideas).

The above example in Rust+proposal syntax:

```
virtual struct Element {
    parent: RC<Element>,
    children: ~[RC<Element>],
    left: RC<Element>,
    right: RC<Element>,
}

impl Element {
    virtual fn foo() -> uint;

    fn template() {
        let x = foo();
        ...
    }
}

virtual struct Element1 : Element {
    x: uint,
    y: uint,
}

impl Element1 {
    override fn foo() -> uint { self.x + self.y }
}

struct Element2 : Element {
    x: uint,
    y: uint,
}

impl Element2 {
    override fn foo() -> uint { self.x + self.y }
}

fn casting_example(el: &Element) {
    // Possible semantics for casting - syntax needs some bike-shedding
    // Idea 1 - `as` returns an Option 
    match el as &Element2 {
        Some(el) => {
            // Use el with type &Element2
        },
        None => {} // el is not an Element2
    }

    // Idea 2 - `if`/`as` idiom
    if el as &Element2 {
        // Use el with type &Element2
    } else {
        // Optional else block for use if el is not an Element2
    }

    // Idea 3 - cast to all possible sub-structs by extending `match`/`as`
    // el is used in all branches but has different types
    match el as {
        e1: &Element1 => {
            // el is an Element1
        }
        e2: &Element2 => {
            // el is an Element2
        }
        _ => {
            // el is something else (el: &Element)
        }
    }
}
```

If a super-struct has an implementation for a trait T, then any sub-structs are
also considered to implement T.

A sub-struct may only inherit from a super-struct in the same module or an outer
module. I.e., the sub-struct must be in a sub-module of the super-struct (where
sub-module is extended to its reflexive, transitive closure). Niko's reasoning:
    1. Prevents people from abusing virtual structs to create an open-ended
    abstraction: traits are more suitable in such cases.
    2. Downcasting is more optimizable, becomes O(1) instead of O(n). This is a
    common complaint against C++ RTTI (as pointed out on the mailing list).
    3. Addresses the private fields initialization gotcha. [see sect. 'struct
    initialisers', below]


# Alternatives

There have been many proposals for alternative designs and variations on this
design. One minor variation would be to use anonymous fields rather than `:`
extension for struct inheritance. An alternative proposal is to allow traits to
extend a single struct and add subtyping appropriately. We would then need to
add support for some kind of RTTI (possibly using a trait and macros) to allow
safe and efficient downcasting.

RFC #9, Fat objects; RFC #11.

# Unresolved questions

Do we need multiple inheritance? We _could_ add it, but there are lots of design
and implementation issues. The use case for multiple inheritance (from bz) is
that some DOM nodes require mixin-style use of classes which currently use
multiple inheritance, e.g., nsIConstraintValidation.

Example with traits:

```
impl Element {
    virtual fn bar() -> uint;
}

trait NSICompositor {
    fn x() -> uint;
    fn y() -> uint;
    fn bar() -> uint { self.x() + self.y() }
}

impl NSICompositor for Element1 {
    fn x() -> uint { self.x }
    fn y() -> uint { self.y }
}

impl Element1 {
    override fn bar() -> uint { NSICompositor::bar(self) }
}

impl NSICompositor for Element2 {
    fn x() -> uint { self.x }
    fn y() -> uint { self.y }
}

impl Element2 {
    override fn bar() -> uint { NSICompositor::bar(self) }
}
```

What to do about (virtual) destructors? I feel the C++ approach is too much of a
foot gun. By limiting struct inheritance to a module, we should always be able to
infer whether or not a destructor is virtual. Need to work out how exactly
implementing the drop trait interacts with inheritance. We need to cope with the
situation where a struct object with static type T1 and dynamic type T2 goes out
of scope and T2 implements `Drop` and T1 doesn't - we still need to call
T2::drop (and then call the destructors of any types between T2 and T1). One
solution could be that if a struct implements `Drop` then so must the base
virtual struct. Calling `drop` is then just a regular virtual call and is only
necessary if the static type implements `Drop`.

What should we do with subtyping between struct refs and inference? Probably,
struct inheritance should give implicit coercion (but not subtyping).


## Struct initialisers

The `S { field:value, ..expr}` initialiser expression should be extended to be
more flexible - possibly allowing multiple `expr`s and allowing the specified
field values to override the field values given in `expr`. We need some way to
specify the fields of a struct that cannot be instantiated (because it has pure
virtual methods). This is required if the struct has `priv` fields and is
specified in another module. We could just not support that case. Or we could
forbid calling virtual methods on struct values and forbid referencing struct
values with pure virtual methods (would this work?).

## Calling overridden methods

If a method is overridden, we should still be able to call it. C++ uses `::`
syntax to allow this, I wonder if we can extend UFCS (#4) to let us do this. So
to call `foo` in element (assuming it were not pure virtual, as in the example)
we would use `self.Element::foo()`.
