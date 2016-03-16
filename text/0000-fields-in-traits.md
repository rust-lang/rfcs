- Feature Name: `fields_in_traits`
- Start Date: 2016-03-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

The primary change proposed here is to allow fields within traits
and then permit access to those fields within generic functions and
from trait objects:

  - `trait Trait { field: usize }`
  - Implementing type can "direct" this field to any lvalue within itself
  - All fields within a trait or its supertraits must be mapped to
    disjoint locations

Fields serve as a better alternative to accessor functions in traits.
They are more compatible with Rust's safety checks than accessors, but
also more efficient when using trait objects.

Many of the ideas here were originally proposed in [RFC 250][RFC250]
in some form. As such, they represent an important "piece of the
puzzle" towards solving the ["virtual struct" problem][RFC349].

# Motivation
[motivation]: #motivation

The change proposed here is to allow field declarations within traits.
These fields are mapped to lvalues within the implementing type.  This
change can be put to many uses:

1. Mix-ins that combine behavioral changes along with a certain amount of
   stage and storage.
2. Alleviating some of the problems that accompany accessors.
3. More efficient trait objects that permit direct field access.
4. Modeling class hierachies; along with [specialization][RFC1210],
   this RFC forms an important step towards a solution to the
   ["virtual struct" problem][RFC349]. (The
   [Future Directions](#futuredirections) section at the end of this
   RFC elaborates on this point.)

Before we dive into the proposal proper, it's worth exploring why the
existing solution for field access in traits (accessor fns) is not
good enough.

### Granting access to fields in traits: accessors are not enough

If you wish to have a trait that exposes access to a field today, the
only way to do is via methods. For example, one could define a trait
with two accessors:

```rust
struct Point { x: f64, y: 64 }

trait GuiNode {
    fn location(&self) -> Point;
    fn set_location(&mut self, p: Point);
}
```

Accessors have a number of disadvantages. Of course there is the
obvious one: they are tedious to define and to use. Writing
`node.location().x` and `node.set_location(p)` is simply less nice
than `node.location.x` and `node.location = p`.  This has led many
languages to adopt syntactic sugar by which the latter can be
automatically translated to the former. However, there are other
disadvantages that are more specific to Rust:

1. Accessors interact poorly with the borrow checker.
2. Accessors have poor performance for trait objects.

#### Accessors interact poorly with the borrow checker

Consider the accessor pair `location` and `set_location` above.
Whenever I call `location`, I get back a freshly owned copy of the
`Point`. This is fine for a type like `Point`, but it could be quite
expensive for fields whose types are not so cheaply cloneable.  For
example, imagine that each `GuiNode` also has a field `children` of
type `Vec<Box<GuiNode>>`. We probably don't want accessors like these:

```rust
trait GuiNode {
    // Probably *NOT* what you want:
    fn children(&self) -> Vec<Children>;
    fn set_children(&mut self, children: Vec<Children>);

    // (Just as before)
    fn location(&self) -> Point;
    fn set_location(&mut self, p: Point);
}
```

In particular, if I have defined my children accessors like those, I
have no choice but to implement them with deep clones:

```rust
impl GuiNode for Layer {
    // Cheap for `Copy` types:
    fn location(&self) -> Point {
        self.location
    }
    fn set_location(&mut self, p: Point) {
        self.location = p;
    }

    // Not so cheap for types like `Vec`:
    fn children(&self) -> Vec<Children> {
        self.children.clone()
    }
    fn set_children(&mut self, children: Vec<Children>) {
        self.children = children;
    }
}
```

Deeply cloning fields every time you want to access them is clearly
too expensive to do in general (though it may be fine in some
instances).

You might think that that it would be better to define the accessors
to return *references* to the vector, rather than cloning the vector
itself:

```rust
trait GuiNode {
    // Returning references is better, but still (as we will see) not great:
    fn children(&self) -> &Vec<Children>;
    fn children_mut(&mut self) -> &mut Vec<Children>;

    // (Just as before)
    fn location(&self) -> Point;
    fn set_location(&mut self, p: Point);
}
```

Indeed, this does solve the deep-cloning problem, but now we have
another problem. Returning references like this interacts somewhat
poorly with the borrow checker rules. In particular, if I call (say)
`children()`, this will effectively "freeze" the `GuiNode` for the
lifetime of the returned reference. In other words, while I am reading
the `children` array, I can't be doing mutating anything *else* with
the `GuiNode`. So, if I wanted to walk the list of children and update
the location as I did so, I would wind up with a borrow checking
error:

```rust
fn tile<G:GuiNode>(node: &mut G) {
    for c in node.children() {
        ... node.set_location() ...  // ERROR
    }
}
```

This is unfortunate, since if I was using an actual struct with real
fields, code like that would be perfectly fine:

```rust
fn tile(node: &mut Layer) {
    for c in node.children {
        ... node.location = x; ...  // OK!
    }
}
```

The reason that the borrow checker accepts the code when written with
actual fields but not when written with accessors is that it knows
*precisely* what reading and writing a field does: it mutates the
memory of the field and that's it. An accessor, in contrast, could do
anything. That is, even though `set_location` *sounds* like it just
updates the `location` field, the borrow checker can't be sure that
it doesn't go off and modify the `children` array too. For example,
maybe there is some GUI node that always keeps its childrens at the
same relative location:

```rust
impl GuiNode for RelocatingLayer {
    ...
    fn set_location(&mut self, p: Point) {
        let relative = p - self.location;
        self.location = p;

        // Adjust locations of children:
        for child in &mut self.children {
            let new_position = child.position() + relative;
            child.set_position(relative);
        }
    }
    ...
}
```

A method like that would mean that our generic function was unsafe,
because we now have both a `&` and `&mut` reference to the same
children array!

#### Accessors have poor performance for trait objects

Another downside of accessors is that you are forced to write method
calls where what you really want is field accesses. In generic code
using bounds, those calls will be statically dispatched, and hence one
can expect inlining. However, when using trait objects, inlining is
typically not possible, which means calls like `child.set_position(x)`
will be significantly more expensive than a field assignment like
`child.position = x`.

This reveals one area where Rust
[lacks expressiveness compared to C++ or Java][bs2]. A traditional
class hierarchy allows you to insert fields (public or otherwise) at
any point. These fields can be accessed with equal efficiency even if
you don't know the exact runtime type of the object. In Rust, in
contrast, you can only access fields if you have a concrete struct
type; whenever you have a trait object, you can only use methods.

[bs1]: http://smallcultfollowing.com/babysteps/blog/2015/05/05/where-rusts-enum-shines/
[bs2]: http://smallcultfollowing.com/babysteps/blog/2015/05/29/classes-strike-back/

#### The tradeoff: flexibility vs performance and safety

There is a tradeoff here. Rust traits, like Java interfaces, were
originally designed for maximum abstraction in mind. On that axis,
accessors are clearly superior, since they allow a "field access" like
`position()` or `set_position()` to require arbitrary computation or
have arbitrary side effects. However, it is precisely that flexibility
which leads to the two downsides cited above:

1. The borrow checker must be more conservative because it does not,
   and cannot, know what that two accessors like `children()` and
   `set_position()` affect disjoint sets of fields.<sup>[1](#endnote1)</sup>
2. The code generated must use a virtual call because it does not,
   and cannot, know how any particular object plans to implement
   `set_children()`.

In a truly generic trait intended for implementation by arbitrary
other types, this tradeoff for maximum flexibility is probably still a
good idea. However, there are many cases where the flexibility to make
accessors do arbitrary things (as opposed to simply access fields) is
really not needed. Examples include traits that are local to a project
(as opposed to public interfaces) as well as
[attempts to model class-like hierarchies][aturon]. It seems best to
leave this decision in the hands of the trait designer.

[aturon]: http://aturon.github.io/blog/2015/09/18/reuse/

# Detailed design
[design]: #detailed-design

### Fields in traits

We extend traits with an optional "field block", which has the same
structure as the contents of a struct (that is, a comma-separated list
of fields; this RFC also adds the notation of a embedded prefix,
described below). If following by further items, the Field block is
terminated by a semicolon, but otherwise the semicolon may be omitted
if the field block is the last item in the trait.

```rust
trait Trait {
    field1: Type1, // <-- fields within a block separated by commas.
    field2: Type2  // <-- no semicolon needed if there are no more items
}

trait Trait {
    field1: Type1,
    field2: Type2; // <-- semicolon terminates the field block

    fn foo();
}
```

**NB.** It might be simpler to just require semicolons after fields, or else
require that they appear at the end of a trait definition. The intention was
that you should be able to copy-and-paste the contents of a struct into a trait.

### Impls with fields

Impls are also extended with a field block, which maps field names to
lvalues:

```rust
impl Trait for Type {
    field1: self.foo.bar,
    field2: self.baz       // <-- optional terminating semicolon, as before
}
```

Trait fields cannot be mapped to arbitrary lvalues. Rather, the lvalue
expression must be an expression of the form `self(.F)*` where `F`
represents some field name (possibly a fully qualified one, see
below). In other words, `self.a.b.c` would be legal, but
`(*self.a.b).c` would not; nor would `self.a[b]`. Furthermore, the
lvalue expression must not contain any implicit derefs.

These rules serve to ensure the following properties:

1. the lvalue is located within the `self` value with some fixed pointer offset;
2. borrowck can easily evaluate whether two field mappings are disjoint.

In the future, we might consider various extensions:

- **The ability to index into fixed-length arrays with a constant
  index.** However, it would be best to couple that with a general
  overhaul of constant evaluation (and probably an extension of
  borrowck to understand expressions of this form more broadly).b
- **The ability to deref.**  This was excluded so as not
  to complicate field access in trait objects, but it could be that
  traits with lvalues that require passing through a reference are
  simply not considered object safe.
  
#### Disjointness rules

The field mappings within an impl must be disjoint from other field
mappings within that same impl, or within super-trait impls. More
specifically, if there is some impl containing a field mapping like so:

```rust
impl<...> Trait<U,V> for T {
   ...
   field: lvalue,
   ...
}
```

then `lvalue` must be *disjoint* (defined below) from the lvalues
found in all other field mappings within that impl. Furthermore,
`lvalue` must be disjoint from the lvalues mapped in supertrait impls.

Note that if a trait has two unrelated supertraits, the fields of those
supertraits do not have to be disjoint. This can be used to form arbitrary
disjointness relationships. For example, imagine that we have some type `Foo`
that implements the trait `C` shown below:

```rust
trait A {
    f: u32,
    g: u32, // must be disjoint from f
}

trait B {
    h: u32, // can overlap with f and g
}

trait C: A + B {
    i: u32, // must be disjoint from f, g, and h
}
```

As the comments suggest, `Foo` could map `A::f` and `B::h` to the same
lvalue if it wanted to, but it must map `A::f`, `A::g`, and `C::i` to
mutually disjoint lvalues.

#### Definition of "disjoint"

Two lvalues are considered "disjoint" if the borrow checker would
allow `&mut` borrows of them simultaneously. This is typically true
because they are based in different fields of the same struct.

### Fully qualified field access

Generally speaking, the lookup for fields will follow the same rules
as methods: if the type itself defines an "inherent" field, that field
is used, but otherwise we search for in-scope traits that implement
the trait and define a field with that name. Sometimes it may be
necessary or desirable to specify the trait explicitly. For those
cases, we introduce a fully qualified field notation which looks like
`x.<Trait<U,V>::f>`.

This is comparable to the associated item notation `<T as
Trait<U,V>>::foo`, but with some differences. First, the `Self` type
is implied by the type of the left-hand side (`x`, in our
example). Second, it is part of an lvalue expression, rather than
being a path to an item.

This is obviously a point where one might bikeshed on the best syntax.
Here are some alternatives that we considered and rejected:

- `x.Trait::f` more closely resembles C++ and Java and could work, but
  it would have to be `x.Trait::<U,V>::f` if you wish to specify the trait's
  type parameters.
- `<x as Trait<U,V>>.f` doesn't work because we'd need a cover grammar for types
  and expressions, since you cannot do not know whether `x` is a type or
  expression until you see the `.`.
- `x::<as Trait<U,V>>.f` seems random

#### Leaving room for method calls

It has not escaped our notice that this same syntax could be used for
method calls. This RFC does not propose that we permit that, but we do
restrict the grammar of call expressions to disallow a fully qualified
field path on the left-hand-side of `()`. In other words, just as
calling a closure located at `a.b.c` must be written `(a.b.c)()` (so
as to avoid ambiguity with calling a method `c` on the path `a.b`), so
must you wrote `(a.b.<Trait::c>)()`; `a.b.<Trait::c>()` will not
parse.

### Field access via traits

This section covers the main rules regarding integrating trait fields
into the borrow checker and other safety analyses.

#### When are two fields disjoint?

Today, if I access two fields of a struct like `base.a` and `base.b`,
those two paths are known to be disjoint by the borrow checker. In the
case where one or both of those fields is defined by a trait, we must
now generalize this notion and define the criteria when two field
names `a` and `b` are considered disjoint from one another (we assume
both are being accessed from the same path `base`):

- Both fields defined in the same struct: disjoint
- Field `a` is defined in a trait `A`, field `b` is defined in a trait `B`:
  disjoint `A` is a supertrait of `B` or vice versa
  (not that every trait is its own supertrait)
- Otherwise: potentially overlapping   

#### Moves are not allowed

For soundness reasons, we disallow moving individual fields out of
values that have a defined destructor. When writing generic code, or
when working with objects, one can never know whether a value has a
destructor or not. Therefore, we disallow moving out of a trait field
if the owner of that field may have a destructor. In practice that means
that moving out from a field owned by one of these sorts of types is
disallowed:

- trait objects (e.g., `Write`);
- generic type parameters (`T`);
- associated type projectons (`T::Output`);

#### Field access and trait objects

Field access through a trait object is permitted. When constructing
the vtable for a trait object, we will compute the offset for each
field within the `Self` type and store it in the vtable. The compiled
code from `object.foo` will thus be to load the offset from the vtable
of `object` and adjust the `object` pointer accordingly. (Note that in
the case of embedded structs, covered below, we can use a more
efficient translation.)

### Privacy

Fields declared in traits are trait items, and hence they are public
to all users of the trait. If you would like to have fields that are not
accessible outside of a module, however, one can embed a struct and make
the fields of that struct private:

```rust
mod x {
    pub struct TraitFields {
        f: u32 // private to `x`
    }
    
    trait Trait {
        fields: TraitFields;
    }
}

mod y {
    fn foo<T: Trait>(t: &T) {
        let value = t.fields.f; // ERROR: `f` is private here.
    }
}
```

#### Private items in public APIs

The rules around private items in public APIs are extended as follows.
Whenever a field that is private to a module `M` is mapped in an impl
of a trait, the trait must also be private to the module `M`. (If the
field is public, that's fine.) For example:

```rust
pub trait Trait {
    x: u32
}

struct Legal {
    pub y: u32
}    

impl Trait for Legal {
    x: self.y
}    

struct Illegal {
    z: u32
}    

impl Trait for Illegal {
    x: self.z // ERROR: Private item in public API
}    
```

This rule is slightly stronger than the rule for associated type
definitions, which states that the value for an associated type must
be public only if the input types to the trait are public. Fields in
traits can only be mapped to public fields of self, full stop. The
reason for the difference has to do with trait objects. Consider what
could happen if we used the same rule that we use for associated
types:

```rust
pub trait Trait {
    x: u32
}

mod foo {
    use Trait;
    
    struct Private {
        z: u32
    }    

    impl Trait for Private {
        x: self.z
    }

    pub fn foo() -> Box<Trait> {
        Box::new(Private { z: 22 })
    }
}

fn bar() {
    let mut obj = foo::foo();
    obj.x += 1; // direct access to the "private" field `z`
}    
```

Here, the private type `Private` escapes as a trait object.  The field
`x` is then available on this trait object. Note that this sort of
thing cannot occur with associated type bindings, because the value of
the associated type must appear in the object type, and so if that
value contains a private type, a compilation error will result. (See
Addendum A for an example.)

<a name="futuredirections"></a>

## Future directions

Although the changes proposed by this RFC stand alone, they were first
considered in [RFC 250][RFC250], which aimed to solve the
["efficient inheritance" problem][RFC349]. While this current RFC
represents a first (and very important) step in this direction, it
does not represent a complete solution. This section discusses some
possible extensions we may wish to consider in the future. There is
also some further discussion (and examples) in
[this blog post][aturon].

### Embedding notation and prefix layout

Early drafts of this RFC included an "embedding" notation that
allowed one struct to embed the fields of another:

```rust
struct Foo {
    /* Foo fields here */
}

struct Bar {
    ..Foo,
    /* add'l bar fields here */
}
```

In addition, this notation could be used in traits:

```rust
trait Baz {
    ..Bar // roughly equivalent to `bar: Bar`, but fields can be accessed directly 
}
```

The original drafts defined `..` to imply "prefix" layout. This made
field access through trait objects particularly efficient (no vtable
access is needed). However, this was removed because it is unclear
whether prefix layout (and efficient trait object access) is important
enough to merit this syntax: perhaps it should be controlled through a
`#[repr]` attribute. Moreover, prefix semantics are less composable
than some other alternatives.

If one does *not* tie `..Foo` to prefix semantics, but instead say
that it just means that the trait is implemented by "some struct that
itself includes a `..Foo` somewhere", this also raises some
interesting questions.  For example, what are the semantics of having
multiple instances of `..Foo` in the same struct? (Perhaps inherited
transitively.) In working out examples, it seemed clear that there was
enough complexity here to merit a distinct RFC (and further
consideration).

In any case, adding `..Foo` notation does not add any expressiveness.
It can always be modeled by using explicit fields, at the cost of some
ergonomics.

### Other changes

To use the pattern in [aturon's blog post][aturon] ergonomically, some
other changes are needed; these changes are largely orthogonal to the
current RFC. Many of these were
[originally proposed in RFC 250][RFC250].

- **Upcasting:** it must be possible to upcast a trait object into a
  "super-trait" object. Currently this is not supported.
  Without this, one cannot pass a `&Container` object to a function that
  expects a `&GuiNode` object.
- **Implicit trait coercion:** it must be possible to invoke an
  inherent method defined on trait `Foo` when given an object of some
  type `T: Foo`. Without this, one could not invoke inherent methods
  defined on the `GuiNode` trait on a `Circle` struct. (This same
  problem affects e.g. the methods on the `Any` trait object.)
- **Thin traits:** For maximum efficiency, we should be able to
  declare a trait as a "thin trait". This would impose stricter orphan
  rules on the trait in exchange for making the size of a trait object
  be a single word (because the vtable can be embedded into the
  implementing types).
- **Super fn definitions:** Specialization currently offers no
  equivalent to the notion of a "super fn" definition from OO
  languages. This means that if, e.g., `Circle` wanted to override the
  `highlight` method but also call out to the prior version, it can't
  easily do so. Super calls are not strictly needed thoug, as one can
  always refactor the super call into a free fn.
  
# Drawbacks
[drawbacks]: #drawbacks

Introducing this feature offers users more choice, but with choice
always comes the opportunity to choose incorrectly. For example, if
one is designing an "open" trait that will be implemented by arbitrary
downstream crates, then embedding a struct with `..N` may well be a
poor choice. That would require that any type which implements the
trait also embeds `N`, which is very limiting. You would be better off
redeclaring the fields within the trait individually, so that
implementors can redirect those fields anywhere within the `Self`
type.

# Alternatives
[alternatives]: #alternatives

## Virtual structs

This RFC is part of an extended conversation about the best way to
model class hierarches in Rust, often called
["virtual structs"][RFC349]. This conversation has been going for
several years now and has spawned numerous proposals and RFCs, far too
many to summarize in full here. Since this RFC is focused on enabling
field access from trait objects, we will just focus our attention on
how other proposals have handled that specific concept.

### Struct bounds

As far back as [this blog post][bssi], the idea of "struct bounds" has
been floating around. Roughly speaking, the idea is to write `struct
Foo: Bar` as the loose equivalent to what this RFC describes as
`struct Foo { ..Bar }` (here `Bar` is a struct). This same bound `Bar`
could also be used on type parameters and traits, so one could write
`fn foo<T: Bar>(t: &T)` to write a function that operates over any
struct which derives from `Bar`. (In fact, an earlier draft of this
RFC adopted this approach, before eddyb convinced me it was
incorrect.)

This approach is convenient, but it is limited to single inheritance.
There is no way to express the idea that a certain set of fields are
available, but not necessarily at the very prefix of a struct. In
contrast, the approach in this RFC allows one to declare a trait that
means "these fields are found in a prefix" as well as a trait that
means "these fields are found somewhere":

```rust
/// This trait can be used when you wish to ensure
/// that fields are located at a prefix.
pub struct Fields { pub f: u32 }
pub trait Prefix {
    ..Fields
}

/// This trait can be used when you just wish
/// to ensure that a field `f` is available somewhere.
trait Somewhere {
    f: u32
}
```

Using traits does imply a certain amount of syntactic overhead. For
example, structs will have to implement the traits to be usable with a
generic fn. But the flexibility seems important and worthwhile (and this
implementation is very lightweight).

### Coercion between sub- and super-struct

Because embedded structs are guaranteed to appear at the prefix,
it is tempting to say that we should be able to coerce a reference to
a struct/trait into a reference to some embedded struct. For example,
given the `GuiNode` trait we defined earlier:

```rust
// Note: these fields are private, since this struct
// is effectively an implementation detail of `GuiNode`.
pub struct GuiNodeFields {
    position: Point,
    children: Vec<GuiNode>,
}

pub trait GuiNode {
    ..GuiNodeFields
}
```

One might assume that a `&GuiNode` or `&mut GuiNode` should be coercible
to a `&GuiNodeFields` or `&mut GuiNodeFields`. This sort of coercion is explicitly not
supported by this RFC because it would enable **object slicing**. For example,
given such coercions, it would then be possible to write a function like:

```rust
pub fn swap_gui_fields(a: &mut GuiNode, b: &mut GuiNode) {
    let a_f: &mut GuiNodeFields = a;
    let b_f: &mut GuiNodeFields = b;
    mem::swap(a_f, b_f);
}
```

This would take two arbitrary `GuiNode` objects and swap their common
fields. Note that the code can do this swap *even though the fields
are private*. However, because those two `GuiNode` objects may
represent distinct kinds of `GuiNodes`, there is no reason to think
that this swap makes sense. For example, some of those common fields
may be a set of flags that are specific to the kind of `GuiNode` that
we are working with, in which case swapping them doesn't make any
sense at all.

What is happening here fundamentally is a kind of object slicing, a
well-known C++ hazard. That is, the type `GuiNodeFields` only
represents a small slice of the fields of a complete `GuiNode`. If you
recall from the "Example", the type `GuiNodeFields` was in fact
intentionally used to represent an incompletely constructed `GuiNode`
object. If we allow coercion from `&mut GuiNode` to `&mut
GuiNodeFields`, we lose the distinction between a completely
constructed `GuiNode` and a reference to its common fields. Note that
this is distinction is also reflected in the fact that `GuiNode` is
not sized whereas `GuiNodeFields` is, precisely because in case of
`GuiNode` we don't know the full set of types that follow.

(Lest you be tempted to think that this hazard is specific to `&mut`
references, the same problem can arise with `&` references that
include cells.)

### Extended enums

An alternative area of exploration for virtual structs has been the
so-called ["Extended Enums"][ee] proposal. This proposal came aim at
the problem from a vastly different angle. Rather than making traits
able to express lower-level features (like the presence of fields), it
extended enums to support higher-level features. For example, each
enum variant became its own type, and we gained the ability to specify
common fields.

Interestingly, that proposal and this RFC are not necessarily in
conflict.  It is true that both of them can be used to model
class-like patterns, but those class-like patterns have quite
different features:

- In the extended enums proposal, the class hierarchy was "closed" --
  it was confined to a single crate.
  - This permits the "base classes" to be sized, if desired, much like
    enum types today. That is, the size of a base class can be made
    equal to the union of the sizes of the substructs.
    - But note that this is not always desirable, so it would be
      important to offer "unsized" enum types as well.
  - This also permits downcasting via exhaustive match, just like
    enums today.
- In contrast, the trait-based approach defines an open hierarchy.
  The "abstract base class" are defined by traits, and those traits
  can be implemented by other crates in the usual fashion.
  - This requires "unsized" base classes, since the size of all subtypes
    in the hierarchy cannot be known.
  - This prevents exhaustive match downcasting.
  
(This dichotomy is essentially a manifestation of the classic
["Expression Problem"][ep].)

Note that the "field in traits" system is compatible with an extended
enum proposal in many ways. For example, if each enum variant were
made into its own type, one could define a trait containing common
fields and have this trait be implemented by the various enum
variants. And so forth.

## Protected qualifier

This RFC proposed to adapt the existing Rust privacy rules in a very
minimal way. Naturally there is lots of precedent for other
approaches.  One obvious feature offered by many OO languages is some
sort of "protected" qualifier, which would make the fields of a struct
`S` available to structs that embed `S`. The protected qualifier
doesn't map very naturally to Rust, since our privacy rules are based
on modules (presumably if a struct `T` embedded another struct `S`,
then the fields of `S` would be available within the module containing
`T`?). Moreover, protected fields don't serve the state purpose of
privacy in Rust: giving a strict bound on the set of files/modules
where one must search for find accesses. Nonetheless, we may find a
need for some kind of "semi-private" privacy construct of this form,
by which downstream crates can gain access to "protected" data in an
upstream crate.

Note that the [`pub(restricted)`][pr] also suggests some stronger
variations on privacy that may be useful in this context, though it
does not offer the "semi-private" status of protected fields. 

## Accessors

Many other languages offer "accessors" -- basically field syntax that
invokes methods. This RFC does not completely preclude the addition of
accessors, but it does make it very unlikely. The motivation section
details why accessors are not deemed to be a good fit for Rust.

# Unresolved questions
[unresolved]: #unresolved-questions

- **Should we permit fields in inherent impls as well?** It seems like
  a natural extension, and might be convenient. For example, one could
  use private field names, and publicly offer a "simpler" set of
  fields. The precise disjointness rules would have to be worked out.
  (For example, can the fields defined in different inherent impls be
  overlapping?)

# Endnotes

<a name="endnote1">1</a>: It has oft been observed that parallelism
and abstraction are at odds. The problem is that to know whether two
functions are safe to run in parallel, one must know what data they
access and how they access it -- but abstraction is all about hiding
those details. Given that [memory safety and data-race freedom][conn]
are both addressed by the borrow checker in Rust, it's not surprising
that accessors -- which preserve abstraction -- would force it to be
more conservative than fields.

[conn]: http://smallcultfollowing.com/babysteps/blog/2013/06/11/on-the-connection-between-memory-management-and-data-race-freedom/
[bbsi]: http://smallcultfollowing.com/babysteps/blog/2013/10/24/single-inheritance/

# Addendums

## Addendum A

An example of how private types cannot escape through associated types.

```rust
pub trait Trait {
    type Out;
}

mod foo {
    use Trait;
    
    struct Private { x: u32 }
    struct AlsoPrivate { y: u32 }
    
    impl Trait for Private {
        type Out = AlsoPrivate;
    }
    
    pub fn foo() -> Box<Trait<Out=AlsoPrivate>> {
        //^ ERROR: Private type `AlsoPrivate` in public API
        Box::new(Private { x: 0 })
    }
}

fn main() { }
```

[RFC1210]: https://github.com/rust-lang/rfcs/pull/1210
[RFC349]: https://github.com/rust-lang/rfcs/issues/349
[bssi]: http://smallcultfollowing.com/babysteps/blog/2013/10/24/single-inheritance/
[RFC250]: https://github.com/rust-lang/rfcs/pull/250
[ee]: http://smallcultfollowing.com/babysteps/blog/2015/08/20/virtual-structs-part-3-bringing-enums-and-structs-together/
[ep]: https://en.wikipedia.org/wiki/Expression_problem
[pr]: https://github.com/rust-lang/rfcs/pull/1422
