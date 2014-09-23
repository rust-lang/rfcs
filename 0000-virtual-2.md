- Start Date:
- RFC PR #: 
- Rust Issue #: 

# Summary

This RFC begins by generalising and unifying enums and structs. We allow
arbitrary nesting of enums/structs and mixing of named and unnamed fields. Inner
enums have any fields defined in outer enums. The `struct` and `enum` keywords
are synonymous. By allowing nesting and enum variants as types, we can allow for
an easy to understand form of refinement types. Data types such as struct
variants fall out naturally. Data structures can be expressed more elegantly
since the programmer does not need to create separate structs to hold the data
for an enum variant if the programmer also wishes to use this data in a stand-
alone way.

We allow optimisation of structs/enums by allowing them to be marked as
`unsized`. Unsized variants may only be referred to by reference, never by
value. This means they can take up a minimal amount of memory, rather than the
maximum of any variant.

Allowing coercions from more deeply nested variants to less deeply nested ones
preserves the current behaviour of enums. We extend this behaviour by
considering that traits implemented by less deeply nested variants are also
implemented by more deeply nested ones. We believe this is the natural behaviour
for such nesting (consider the case for enums today if we could impl for
individual variants, it is natural to assume impls for enum apply to the
variants too).

As a futher optimisation, we allow marking traits and concrete data types as
`closed`. This prevents implementations from outside the crate in which they are
declared, but allows the compiler to optimise the representation of trait
objects as thin pointers with inline vtables, rather than fat objects with
vtables kept with the pointer.

The changes to the data types can be considered as introducing 'data
inheritance'. In keeping to the Rust design principle of keeping data and
behaviour separate, this data inheritance is separate from the behaviour
inheritance between traits.

These features taken together allow for very efficient implementation of data
structures such as the Servo DOM or the Rust AST.

These changes are mostly backwards compatible, see the staging section for more
details.


# Motivation

Supporting efficient, heterogeneous data structures such as the DOM or an AST
(e.g., in the Rust compiler). Precisely, we need a form of code reuse which
satisfies the following constraints:

* cheap field access from internal methods;
* cheap dynamic dispatch of methods;
* cheap downcasting;
* thin pointers;
* sharing of fields and methods between definitions;
* safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.

# Detailed design

Syntactically, we unify structs and enums and allow nesting. That means enums
may have fields and structs may have variants. Both may have nested data; the
keyword (`struct` or `enum`) is only required at the top level. Unnamed fields
(tuple variants/tuple structs) are only allowed in leaf data and only if inner
variants have no data (I expect this rule could be relaxed somehow in the
future). All existing uses are preserved. Some examples:

plain enum:

```
enum E1 {
    Variant1,
    Variant2(int)
}

let x: E1 = Variant1;
let y: E1 = Variant2(4);
```

plain struct:

```
struct S1 {
    f1: int,
    f2: E1
}

let s: S1 = S1 {f1: 5, f: y};
```

enum with fields:

```
enum E2 {
    f: int,
    Variant1,
    Variant2{f2: int}
}

let x: E2 = Variant2{f: 34, f2: 23};
```

nested enum:

```
enum E3 {
    Variant1,
    Variant2(int),
    VariantNest {
        Variant4,
        Variant5
    }
}

let x: E3 = Variant1;
let y: E3 = Variant4;
```

nested struct:

```
struct S1 {
    f1: int,
    f2: E1,
    S2 {
        f3: int
    }
}

let s1 = S1 {f1: 5, f: y}
let s2 = S2 {f1: 5, f: y, f3: 5}
```

All names of variants may be used as types (that is, from the above examples,
`E3`, `Variant1`, `VariantNest`, `Variant5`, `S1`, `S2` may all be used as
types, amongst others). Fields in outer items are inherited by inner items
(e.g., `S2` objects have fields `f1` and `f2`). Field shadowing is not allowed.

We allow logical nesting without lexical nesting by using `:`. In this case a
keyword (`struct` or `enum`) is required and must match the outer item. For
example, `struct S3 : S1 { ... }` adds another case to the `S1` defintion above
and objects of `S3` would inherit the fields `f1` and `f2`. Likewise, one could
write `enum Variant3 : E3;` to add a case to the defintion of `E3`. Such items
are only allowed in the same crate as the outer item. Why?

    1. Prevents people from abusing virtual structs to create an open-ended
    abstraction: traits are more suitable in such cases.
    2. Downcasting is more optimizable, becomes O(1) instead of O(n). This is a
    common complaint against C++ RTTI (as pointed out on the mailing list).
    3. Prevents the 'fragile base class' problem - since there are no derived
    structs outside the base struct's crate, changing the base class has only
    limited and known effects.

All leaf variants may be instantiated. A non-leaf variant may only be
instantiated if it has no inline children. Whether or not it has out of line
children is irrelevant. (e.g., you can't create an `E3` or `VariantNest`
object). In any case, the variant may be used in pattern matching. This might
seem an odd rule, but I believe it fits the common use cases for structs/enums
without requiring the two keywords to have different behaviour and maintains
backwards comparability.

A variant may be marked as `abstract` which means it may not be instantiated. It
is an error to mark a leaf variant as `abstract`. Marking a variant which may
not be instantiated as abstract should give a warning (a configurable lint).

**Open question:** should `abstract` be a keyword or an attribute? I am leaning
towards attribute.

When pattern matching data types, you can use any names from any level of
nesting to cover all inner levels. E.g.,

```
fn foo(x: E3) {
    // All three versions give correct coverage
    match x {
        E3 => {}
    } 
    match x {
        Variant1 => {}
        Variant2(_) => {}
        VariantNest => {}
    } 
    match x {
        Variant1 => {}
        Variant2(_) => {}
        Variant4 => {}
        Variant5 => {}
    } 
}
```

Enums/structs may be annotated with `[#unsized]`. Enums/structs without this
annotation are represented as enums are today. They have a tag and are the size
of the largest variant plus the tag. A pointer or reference to an enum object is
a thin pointer to a regular enum object. Nested variants should use a single tag
and the 'largest variant' must take into account nesting. Even if we know the
static type restricts us to a small object, we must assume it could be a larger
variant. That allows for trivial coercions from nested variants to outer
variants. We could optimise this later, perhaps.

Non-leaf variants which are unsized follow the rules for DSTs. A programmer
cannot use non-leaf variants as value types, only pointers to such types may
exist. E.g., (given the definition of `S1` above) one can write `x: S2` (since
it is a leaf struct), `x: &S2`, and `x: &S1`, but not `x: S1`. Unsized values
have their minimal size (i.e., their size does not take into account other
variants). This is the current behaviour for structs. Pointers to unszied
variants are DST pointers, but are not fat. They point to a pointer to a vtable,
followed by the data in the variant. The vtable pointer allows identification of
the concrete variant.

Pointer-to-unsized-variant objects may only be dereferenced if the static type
is a leaf variant. Dereferencing gives only the concrete object with no
indication of the vtable.

For example,

```
enum E {
    EVar1,
    EVar2,
    EVar3(i64, i64, i64, i64),
}

[#unsized]
struct S {
    SVar1,
    SVar2,
    SVar3(i64, i64, i64, i64),
}

fn foo_e(e: E) {...}
//fn foo_s(s: S) {...}

fn foo_er(e: &E) {...}
fn foo_sr(s: &S) {...}

fn foo_e1(e: EVar1) {...}
fn foo_s1(s: SVar1) {...}
```

Here an instance of `EVar1` has size 40 bytes (assuming a 64 bit discriminator)
and an instance of SVar1 has zero size. An instance of `&E` has size 8 bytes
(assuming 64 bit pointers) and points to an object of size 40 bytes. An instance
of `&S` has size 8 bytes and pointers to an object of unknown size. If the
dynamic type is `&SVar1` then the pointed-to data has size 8 bytes (zero size
value plus a pointer to a vtable).

The function `foo_s` is a type error because `S` is an unsized type (DST). The
other functions are all valid.

A programmer would typically use sized enums for small, similar size objects
where the data is secondary and discrimination is primary. For example, the
current `Result` type. Unsized enums would be used for large or diversely sized
objects where discrimination is secondary, that is they are often used in a
polymorphic setting. A good candidate would be the AST enum in libsyntax or the
DOM in Servo.

**Open question:** we should have style guidlines for when to use `struct` vs
`enum` keywords.

Matching unszied objects (that is pointer-to-unsized-variants) should take into
account the dynamic type given by the vtable pointer and thus allow for safe and
efficient downcasting.


## Methods and the closed annotation

We allow impls for any enum or struct anywhere in the nesting tree of the data
type (since they are all valid types). If an outer data structure implements a
trait, then all of its children are considered to implement that trait.
Furthermore, if any of the children implement that trait, they are not obliged
to provide all the required methods. Any missing methods use the definitions for
the parent data structure.

Note: I believe this arrangement is natural for enums if we allow variants to be
types and thus have impls. We then extend this mechanism to structs, since they
should have identical behaviour to enums.

An impl may be marked as `abstract` (e.g., `abstract impl T for U { ... }`).
This means that the impl does not need to provide all methods required by the
trait. The point of this is to provide default implementations of some trait
methods and allow the rest to be provided by a child data type. To prevent calls
to 'pure virtual methods', only concrete data which cannot be instantiated may
have abstract implementations.

**Open question:** alternative for `abstract`: `virtual`. We could also not
require a keyword and infer the `abstract`-ness from whether or not all methods
are provided. That has the effect of pushing errors from the impl to trait
mathcing. But it does mean less annotation.

We introduce an attribute for traits and concrete types: `closed`. Any trait may
be marked as closed and this means that it may only have impls in the same crate
as it is declared. Furthermore, only concrete types in that crate and marked as
`closed` may implement a closed trait. In turn that means that trait objects for
closed traits may be optimised as thin pointers. Other than that, there is
nothing special about inheritance or impls for closed traits.

Any closed concrete data type has a vtable pointer as a first field in its
memory layout (note that nested structs/enums would have such a field in any
case, other data structures would get an extra field). The vtable is only used
for methods dispatch on trait objects, as we do today.

There is a bit of an edge case for enums here. Enum variants have a tag in their
representation to identify the variant. We could re-purpose this slot as a
vtable pointer to allow nested enums to implement closed traits without adding
an extra word to their representation. However, that would preclude any of the
other optimisations we do for enums. I would suggest that if an enum is marked
closed, then it gets a vtable pointer in all cases, i.e., is not eligible for
the other enum optimisations. We might be able to make this less strict in the
future.

A trait object for a closed trait is always a thin pointer. If a closed
concrete data type value is coerced to a non-closed trait object, it is
represented as a fat object.

**Open question:** I realised that this approach is actually a bit more subtle
than I appreciated, so there are a few details to work out:

* The assumption so far has been that unsized variants get a vtable ptr and
 sized variants get a type tag (or some optimised equivalent). I think that is
 not quite right. The sized-ness doesn't affect the kind of tag. If a variant
 is closed it must get a vtable (since it may be used with closed traits). If
 it is not closed it should get a vtable if it has overriding methods in impls
 (as described above). Alternatively, we could only allow overriding methods
 for closed variants (this seems the sanest-option, although overriding has
 nothing to do with closed-ness) or add another attribute to allow overriding
 (this might be preferable since overriding has engineering concerns as well as
 implementation ones). If we don't do something like this we end up with
 dynamic dispatch for unsized data and static dispatch for sized data, which
 seems highly undesirable.

* Note that if we take self by value in a method, we will get static dispatch in
 any case, but I think that is OK.

* We must forbid `&mut self` for sized variants being used as a default method
 (i.e., they must not be inherited). This is for the same reason as we avoid
 coercion between `&mut` objects. Alternatively, we could always treat `Self`
 as unsized for `&mut self` methods when the method is inherited. Neither
 option seems nice, but I don't think we can avoid `&mut self` entirely.

* Methods with a `Self` type other than in `self` position can also not be
 inherited (because the type of Self changes). Alternatively we could allow
 `Self` in method implementations and type check the method for all values of
 `Self` (seems like future work).

* Methods with type parameters can't be overridden because of monomorphisation
 issues.

This all sounds like a lot of subtle and horrid complexity. I hope there is some
way to slice the Gordian Knot, rather than requiring all these fiddly rules.

**end of open question**


## Subtyping and coercion

Nothing in this RFC introduces subtyping.

Inner sized variant values can implicitly coerce to outer variant values as can
sized pointer values.

Inner unsized pointer values can implicitly coerce to outer variant pointer
values. Note that there is no coercion between unsized values. Unsized variant
pointers may not be dereferenced. Thus we are immune to the object slicing
problem from C++. Sized mutable references cannot be upcast (coerced) in this way
since it would be unsafe (once the scope of the 'super-type' borrow expires, the
'sub-type' reference can be accessed again and the pointee may have changed
type). Coercion of mutable `Box` pointers is allowed.

Via the DST rules, it should fall out that these coercions work for smart
pointers as well as `&` and `Box` pointers.

If in the future we decide that subtyping is useful, we could add it backwards
compatibly.

## Generics

When using the inline syntax for enums and structs, only the outermost variant
may have formal type parameters. All inner variants take the same type
parameters. Examples:

```
[#unsized]
struct Sg<X, Y> {
    Sgn2 {
        field: Foo<X>
    }
}

let x = Sgn2<int, int> { field: ... };
```

When non-nested syntax is used, all type parameters must be specified, including
actual type parameters for the parent. (Note also that the super-type is named
whether or not type parameters are present). E.g.,

```
[#unsized]
struct Sg<X, Y> {}

struct Sgn<X, Y, Z> : Sg<Y, X> {
    field: Foo<X, Z>
}

let x = Sgn<int, int, int> { field: ... };
```

If the struct/enum is sized, then all variants must take the same parameters.
This is automatically the case when variants are declared inline. For out of
line variants, the actual and formal parameters must be the same (e.g., `struct
Sgn3<X, Y> : Sg<X, Y>`). This is necessary both for backwards compatibilty and
to know the size of any variant instance.

## Privacy

The privacy rules for fields remain unchanged. Nested items inherit their
privacy from their outer-most parent, i.e., module private by default unless the
parent is marked `pub`.

**Open question:** is there a use case for allowing nested items to be marked
`pub`? That is having a private parent but public child. What about the
opposite?


## Drop

Traits may be marked `inherit`: `inherit trait Tr {...}` (this keyword could
also be an attribute). This implies that for an item `T` to implement `Tr` any
outer item of `T` must also implement `Tr` (possibly providing an abstract
declaration if the outer item is itself abstract). This is checked where the
impl is declared, so it would be possible that an impl could be declared for an
outer item in a different module but due to the visibility rules, it is
invisible, this should be a compile error. Since `impl`s are not imported, only
traits, I believe this means that if a trait is marked `inherit`, then anywhere
an implementation for an inner item is visible, then an implementation for the
outer item is also visible.

`Drop` is marked `inherit`.

Where an object goes out of scope, the compiler will check for the Drop trait
like it does today. However, if it finds one on the static type, then it will
generate code which calls all implementations of drop up the inheritance
hierarchy (rather than calling a single destructor). Note that by marking the
`Drop` trait as `inherit`, it is not possible that the dynamic type has a
destructor, but the static type does not.

I believe this is possible by walking the vtable and calling all methods rather
than just the first. So this should not require any additional reflection
capabilty.

I believe that this gives the desired behaviour and is backwards compatible,
other than the addition of the `inherit` keywords. It is the desired behviour
for destructors, but it is a little bizarre when thought of in terms of regular
virtual method calls. I think this is the least worst option, however.

A possible generalisation of this is a mechanism for requiring inner items
to implement a trait (with or without implementing it in the outer item, the
former case is like saying "must override")? This is kind of dual to the idea
above that if an outer item implements a trait, then the inner trait appears to
implement it too, via coercion. (ht Niko).

**Alternative:** don't use the inherit machinery described above and only do
the above checks for the `Drop` trait as a special case. This is less general,
but less complex. I'm not sure if there are any use cases for `inherit` other
then `Drop`.


## Trait matching

When searching for traits to match an expression, super-structs/enums should
also checked for impls. Searching for impl candidates is essentially a matter of
dereferencing a bunch of times and then trying to apply a subset of coercions
(auto-slicing, etc.), and then auto-borrowing. With this RFC, we would add
checking of outer items to the set of coercions checked. We would only consider
these candidates for unsized variants if the type of `self` is a reference type.


# JDM's example

From https://gist.github.com/jdm/9900569

```
// closed means we can downcast via match and optimise to a thin pointer
#[closed]
trait Node {}

#[closed]
trait Element {
    fn set_attribute(&mut self, key: &str, value: &str);
    fn before_set_attr(&mut self, key: &str, value: &str);
    fn after_set_attr(&mut self, key: &str, value: &str);
}

// This is here just to show how a sub-trait works with this
#[closed]
trait MediaElement : Element {
    fn display_media(&mut self, dest: &MediaDestination);
}

#[closed, unsized]
struct NodeData {
    parent: Rc<Node>,
    first_child: Rc<Node>,

    abstract ElementData {
        attrs: HashMap<str, str>    
    }
}

// abstract here means NodeData can't be an Element itself, but can help its children
abstract impl Element for NodeData {}

abstract impl Element for ElementData {
    fn set_attribute(&mut self, key: &str, value: &str)
    {
        self.before_set_attr(key, value);
        //...update attrs...
        self.after_set_attr(key, value);
    }
}

// Note these structs don't need #[closed,unsized] since they extend a struct
// with those annotations.
struct TextNode : NodeData {}

struct HTMLImageElement : ElementData {}

impl Element for HTMLImageElement {
    fn before_set_attr(&mut self, key: &str, value: &str)
    {
        if (key == "src") {
            //..remove cached image with url |value|...
        }
        // TODO not clear what this is meant to do since Element::before_set_attr
        // is pure virtual in the C++ version.
        ElementData::before_set_attr(self, key, value);
    }    
    fn after_set_attr(&mut self, key: &str, value: &str) { ... }
}

impl MediaElement for HTMLImageElement {
    fn display_media(&mut self, dest: &MediaDestination) {
        self.set_attribute("displaying", "true");
    }
}

struct HTMLVideoElement : ElementData {
    cross_origin: bool
}

impl Element for HTMLVideoNode {
    fn before_set_attr(&mut self, key: &str, value: &str) { ... }
    fn after_set_attr(&mut self, key: &str, value: &str)    
    {
        if (key == "crossOrigin") {
            self.cross_origin = value == "true";
        }
        ElementData::after_set_attr(self, key, value);
    }
}

impl MediaElement for HTMLVideoNode {
    fn display_media(&mut self, dest: &MediaDestination) {
        if self.cross_origin {
            dest.foo();
        }
    }
}

fn process_any_element(element: &Element) {
    // ...
}

fn foo() {
    let videoElement: Rc<HTMLVideoElement> = ...;
    process_any_element(&*videoElement);

    let node = videoElement.first_child;

    // Downcasting
    match node {
        element @ box (Rc) TextNode{..} => { ... }
        _ => { ... }
    }
}
```


# Drawbacks

We are adding a fair bit of complexity here, in particular in allowing nesting
of structs/enums. The reduction in complexity by unifying structs and enums has
clearer advantages to the language implementation than to users of the language.


# Alternatives

See http://discuss.rust-lang.org/t/summary-of-efficient-inheritance-rfcs/494

TODO struct vs enum difference

# Unresolved questions

What to do if there are implementations for a data structure for multiple,
unrelated, closed traits? Should this be an error? Or should we fall back to fat
pointers (I don't think this will work, because users of the trait objects will
expect a thin pointer) with a warning? Or can we emulate C++ vtables to give
efficient multiple inheritance?


## Initialisation

To initialise an unsized struct you must give values for all its fields. There
is a technical and an ergonomic problem here: if the base struct is in a
different module, then its private fields cannot be named in the constructor for
the derived struct; and if the base struct has a lot of fields, it is painful
and error-prone to write out their values in multiple places.

We can address the first problem by adjusting the privacy rules to always allow
the naming of private fields in constructors if the most derived struct's fields
are visible.

To address the second problem, we start by adjusting the rules for struct
initialisers. An initialiser currently has the form
`Foo { f_0: value_0, ..., f_n: value_n, .. e }` where `e` is an expression with
type `Foo` and which supplies any fields not in the field list. We can make this
more general by accepting an expression with type `Foo` or any of its base
structs, where the programmer must explicitly give at least any missing fields.
This addresses both the first and second problems described above. However, it
has a problem of its own if the base struct is abstract - then we cannot
create an instance with the required type, so the derived struct is impossible
to instantiate.

I don't see any good way to solve this problem. Here are some ideas (I think the
second or third are my favourites):

* Where a struct is abstract, allow the struct to be instantiated, but do not
  allow any method calls on such objects, nor taking its address. The only
  operations allowed on objects with such type are field access and use in
  initialiser expressions. This is yet more added complexity.

* Some attribute similar to `deriving` for a virtual base struct which
  automatically generates a non-virtual derived struct with no extra fields and
  with an implementation which `fail!`s for every method call. A constructor for
  the base struct can create one of these objects and return a `Box<Foo>` (if
  `Foo` is the base struct). We would adjust struct initialisers to allow
  `Box<T>` as well as `T` where `T` is the struct being initialised or any of
  its base structs. This can be thought of as a dynamic version of the above
  (static) proposal. It has the advantage of much less compiler complexity and
  no new language rules (ish), however, this comes at the expense of risk of
  runtime failure.

* Allow fields to have default values, e.g., `struct Foo { x: int, y:int = 42 }`,
  here `y` has a default. When instantiating `Foo`, `x` must be provided and `y`
  may be. If `y` is provided then it overrides the default value. If a struct is
  to have derived structs in different modules, then all private fields must
  have defaults (this does not need to be enforced by the compiler - it is a
  natural consequence). This has the disadvantage that fields can only ever have
  a single default (as opposed to using multiple constructor functions) and
  there can be no input to field defaults. It is also more language complexity,
  but I believe this is useful in regular structs too.


## Multiple inheritance

Do we need multiple inheritance? We _could_ add it, but there are lots of design
and implementation issues. An example use case for multiple inheritance (from bz) is
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

I believe that all such uses can be implemented using the traits mechanisms in
Rust and that these will interact cleanly with the rest of this RFC. Therefore,
we should not add any additional mechanism for multiple inheritance.


# Staging

Follows approximately the plan laid out in #142.

Note that allowing enum variants as types introduces a backwards incompatibility
due to type inference - type inference will infer the most minimal type, if that
changes to be a variant rather than the whole enum, we could get errors. For
example, `let v = vec![Some(2i), Some(3)]`; today, `v` would have inferred type
`Vec<Option<int>>`, with these changes, it could be inferred the type
`Vec<Some<int>>`. There would then be an error with `v.push(None)` which is not
an error today.

To avoid this backwards incompatibility we could make enum variants valid types
before 1.0.
