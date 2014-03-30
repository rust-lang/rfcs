- Start Date: 2014-03-31
- RFC PR #: 
- Rust Issue #: 


# Summary

Unify enums and structs by allowing enums to have fields, and structs to have
variants. Allow nested enums/structs. Virtual dispatch of methods on struct/enum
pointers. Remove struct variants. Treat enum variants as first class. Possibly
remove nullary structs and tuple structs.

The motivation for this is to provide an alternative to Java-style single
inheritance. I.e., efficient sharing of fields, thin pointers, and virtual
method dispatch. Along the way we simplify the language by unifying two language
items and making obsolete a few more.

Despite being a fairly radical proposal, I believe this is mostly backwards
compatible.

# Motivation

Supporting efficient, heterogeneous data structures such as the DOM. Precisely
we need a form of code sharing which satisfies the following constraints:

* Cheap field access from internal methods;
* Cheap dynamic dispatch of methods;
* Cheap downcasting;
* Thin pointers;
* Sharing of fields and methods between definitions;
* Safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.

Example (Java-like pseudo-code):

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

## Extend enums with fields

For example,

```
enum E {
    f1: T1,
    Var1(T5, T6),
    f2: T2
}
```

## Extend structs with Variants

For example,

```
struct S {
    f3: T3,
    Var2(T5, T6),
    f4: T4
}
```

## I.e., unify structs and enums

With the above extensions, enums and structs are basically the same (they have
the same syntax (modulo the keyword), we would allow the same type
parameterisation, etc.). The difference is that only structs can be instantiated
(as opposed to one of the variants; you could think of enums being abstract
structs). So we could have values of `Var1` and `Var2` (but not `f1`, etc.), and
`S`, but not `E`. When instantiating `S`, we must specify values for fields `f3`
and `f4`. Values of `Var1` have named fields `f1` and `f2` and unamed fields of
types `T5` and `T6`, all must be specified when instantiating `Var1` (questions
- what should the syntax look like? How do we specify constructors for the `E`
part?).

## Allow nested enums/structs

For example,

```
enum E1 {
    enum E2 {
        ...
    }
    struct S1 {
        enum E3 { ... }
        struct S2 { ... }
    }
}
```

Nesting does not introduce a scope, so from the same scope as `E1` is declared,
we can refer to `E1`, `E2`, `S1`, `E3`, and  `S2` (modulo privacy, see open
questions). Nested items inherit fields from outer items. So, `S2` would inherit
fields declared in `E1` and `S1`.


## Treat variants as 'first class'

As well as instantiating variants, we allow the use of variants (whether
structs, enums, tuples, or nullary) as types and allow impls for them. In
combination with nested enums this is a partial replacement for 'refinement'
types (that is, specifying a type on a subset of the variants of an enum).
However, this is not the main motivation. The idea is that a variant (probably a
struct variant) will replace a base class in a class hierarchy; an enum would
replace an abstract base class and a struct would replace a non-abstract base
class or leaf (concrete) class. Making variants first class makes it possible to
refer to enum/struct objects other than the top level by type, and to provide
methods for them in impls.


## Virtual dispatch of methods for struct/enum objects

We allow methods in impls for struct/enum objects (that is, references to
struct/enum types) to be marked as `virtual` (allows overriding) and/or
`override` (overrides a method). Methods declared on outer items are inherited
by nested iterms. E.g., from the example above, a method declared on `E1` will
be inherited by `E2` and `S2` (and others). If a method is declared `virtual`,
then impls for nested items may override that method. If and only if a method is
marked `override` then it must override a method declared in an outer item.
Methods for enums may be declared without a body (as pure abstract/virtual
methods in Java/C++ or required methods on traits). These must be overriden by
any non-enum nested items. (Question - should we extend this to structs - i.e.,
allow pure virtual methods for structs and track these and not allow
instantiation of such structs?).


## V-tables, thin pointers, and down-casting

Struct/enum objects are referred to using thin pointers. Virtual dispatch is
implemented using Java-style (or C++ with virtual single inheritance and without
multiple inheritance) v-tables. That is, `&S1` or `~S1` is implemented as a
pointer to a structure consisting of a pointer into a v-table (which identifies
the dynamic type) and values for all fields of the dynamic type. Method call is
implemented via the v-table. Since we identify the dynamic type, we can allow
safe dynamic downcasting. This can be done by a match statement, continuing the
example above:

```
fn f(x: &E1) {
    match x {
        y @ &S2 {..} => { ... } // y is effectively a downcast of x to S2
        y @ &S1 {..} => { ... } // y is effectively a downcast of x to S1
        _ => { ... } // x isn't an instance of S1 or S2
    }
}
```

We would allow the usual pattern matching too.

Question - might be handy to allow skipping the `{..}` for structs, then again,
hopefully downcasting won't be commonly used so maybe we don't need to.


## Remove struct variants

Unification of structs and enums makes struct variants obsolete. For example,

```
enum E {
    Variant1{f: T}
}
```

can be written as

```
enum E {
    struct Variant1{f: T}
}

```

Therefore, we may as well remove struct variants (they are currently
feature-gated).


## Coercions (subtyping)

Nesting of enums/structs should give (probably implicit) coercions of
references. E.g., (again, from the above example), `&S2` <: `&S1` <: `&E1`.
There is no subtyping between values, to avoid the slicing problems (er, is this
right? Or my imagination? I think we do get into problems with the expectation
of virtual dispatch, but not being able to, safely, but probably I need to think
more about this).

We should forbid dereference of pointers to non-leaf items. This is not
backwards compatible, since for a non-nested enum (as currently present in the
language), we would allow dereference of references to such enums. We could
safely allow dereference inside a match expression (as in the downcast example,
above) and hopefully that covers most of the current use cases. This would need
a bit of investigation.


# Example

The first example in Java-ish syntax would be written as:

```
enum Element {
    parent: RC<Element>,
    children: ~[RC<Element>],
    left: RC<Element>,
    right: RC<Element>,

    struct Element1 {
        x: uint,
        y: uint,
    },

    struct Element2 {
        x: uint,
        y: uint,
    }    
}

impl Element {
    virtual fn foo(&self) -> uint;

    fn template(&self) {
        let x = self.foo();
        ...
    }
}

impl Element1 {
    override fn foo(&self) -> uint { self.x + self.y }
}

impl Element2 {
    override fn foo(&self) -> uint { self.x + self.y }
}
```

None of this prevents the usual use of traits and impls, which hopefully are an
alternative to multiple inheritance. For example, `nsIConstraintValidation` is a
mixin class in the Gecko DOM implementation. It could be implemented in Rust
as something like:

```
impl Element {
    virtual fn bar(&self) -> uint;
}

trait NSICompositor {
    fn x(&self) -> uint;
    fn y(&self) -> uint;
    fn bar(&self) -> uint { self.x() + self.y() }
}

impl NSICompositor for Element1 {
    fn x(&self) -> uint { self.x }
    fn y(&self) -> uint { self.y }
}

impl Element1 {
    override fn bar(&self) -> uint { NSICompositor::bar(self) }
}

impl NSICompositor for Element2 {
    fn x(&self) -> uint { self.x }
    fn y(&self) -> uint { self.y }
}

impl Element2 {
    override fn bar(&self) -> uint { NSICompositor::bar(self) }
}
```


# Alternatives

RFC 5 - virtual structs

RFC 11 - Alternative to virtual struct and functions by extending enums

RFC 9 - RFC for "fat objects" for DSTs

There's also a version of RFC 5 using macros etc. to add fewer language features.


# Unresolved questions

## Trait methods

I think requiring indication of overridable and overriding methods is a good
thing (both Java and C++ have keywords or annotations for this). However, we
don't require them for methods in traits - should we? Or should we not require
them for structs/enums for consistency? If we do want them for traits should
they be in the trait or the impl? Trait seems to make more sense, but impl is
what I propose here for structs/enums. I would like to have a consistent story
here.


## Remove tuple structs, nullary structs

Unifying structs and enums and making variants first class makes enum structs
and empty structs obsolete. They can be replaced by an enum with a single tuple
variant or a single nullary variant, respectively. By combining with privacy
annotations we might get a nice separation between interface and implementation.
On the other hand it requires an extra name (maybe we should allow anonymous
enums?) and a bit more syntax. One use case for tuple structs is new types. Not
sure if the interface/implementation separation helps there or whether the extra
`enum` keyword, name, and braces are just extra boilerplate. I think removing
some language items would be nice.


## Privacy

I think all fields should be private by default on enums and structs, and
variants should be public. We should allow `pub` and `priv` annotations to
change these defaults. But we need to think about this a bit more deeply.


## Destructors

How should they work? I feel the C++ approach is too much of a foot gun. We
should always be able to infer whether or not a destructor is virtual. Need to
work out how exactly implementing the drop trait interacts with nested enums. We
need to cope with the situation where a struct/enum object with static type T1
and dynamic type T2 goes out of scope and T2 implements `Drop` and T1 doesn't -
we still need to call T2::drop (and then call the destructors of any types
between T2 and T1). One solution could be that if a struct implements `Drop`
then so must the outer struct/enum. Calling `drop` is then just a regular
virtual call and is only necessary if the static type implements `Drop`.

## Initialisers

Need to think a bit about struct initialisers. We should require all fields to
be specified. We should support constructors too. I'm not sure how we support
'struct' initialisers for enums - which should not be instantiable. Since there
is no kind of cross-module inheritance, perhaps it is not an issue since fields
can always be accessed.

## Calling overridden methods

If a method is overridden, we should still be able to call it. C++ uses `::`
syntax to allow this. In the example above we use `Foo::bar(self)` to indicate
static dispatch of an overridden method. I'm not sure if this is currently
valid Rust or if it is the optimal tsolution. But it looks nice to me and we
need something for such a situation.

## Generics

Not sure exactly how generics would work right now. I assume generics in outer
items are available (and not overridable/shadowable) in inner items. All actual
type parameters must be specified or inferred when an item is instantiated or
used for a type (which is a little counter-intuitive). E.g.,

```
struct S1<X> {
    struct S2<Y> {
        ...
    }
}
```

When we use `S2` we would have to use `S2<T1, T2>`. Or perhaps we should say we
require at least as many type variables in inner items as outer and implicitly
substitute and outer type variables are not available inside inner items (i.e.,
in the example above, `X` and `Y` are implicitly linked and `X` can't be used
inside `S2`. We would use `S2<T>`). Or perhaps we should make the substitution
explicit somehow (this would be my preferred solution, but I'm not sure how to
express it).
