- Feature Name: `untagged_unions` (v1.2)
- Start Date: 2015-12-29
- RFC PRs: https://github.com/rust-lang/rfcs/pull/1444,
           https://github.com/rust-lang/rfcs/pull/1663,
           https://github.com/rust-lang/rfcs/pull/1897
- Rust Issue: https://github.com/rust-lang/rust/issues/32836

<!-- TOC -->

- [Summary](#summary)
- [Motivation](#motivation)
- [Overview](#overview)
- [Detailed design](#detailed-design)
    - [Definitions](#definitions)
        - [Union variant](#union-variant)
        - [Trivially destructible type](#trivially-destructible-type)
        - [Structural fragments](#structural-fragments)
        - [Active fragment and active field](#active-fragment-and-active-field)
        - [Initialization state](#initialization-state)
    - [Union declaration](#union-declaration)
        - [Contextual keyword](#contextual-keyword)
        - [Representation attributes](#representation-attributes)
        - [Dynamically sized fields](#dynamically-sized-fields)
        - [Unions and `#[derive]`](#unions-and-derive)
    - [Constructing union values](#constructing-union-values)
    - [Accessing union fields](#accessing-union-fields)
        - [Field access (`.`)](#field-access-)
        - [Pattern matching](#pattern-matching)
    - [Unions in constant expressions](#unions-in-constant-expressions)
    - [Unions and special traits](#unions-and-special-traits)
        - [Unions and `Drop`](#unions-and-drop)
        - [Unions and `Copy`](#unions-and-copy)
    - [Ownership](#ownership)
        - [Borrow checking](#borrow-checking)
        - [Move and initialization checking](#move-and-initialization-checking)
    - [Guarantees and undefined behavior](#guarantees-and-undefined-behavior)
        - [Requirements and desirable properties](#requirements-and-desirable-properties)
            - [Type punning](#type-punning)
                - [Layout compatibility at type level](#layout-compatibility-at-type-level)
                - [Layout compatibility at value level](#layout-compatibility-at-value-level)
                - [History of assignments](#history-of-assignments)
                - [Reading uninitialized bits](#reading-uninitialized-bits)
            - [Other desirable properties](#other-desirable-properties)
                - [Ownership issues](#ownership-issues)
                - [Reading the active field](#reading-the-active-field)
        - [Guarantees](#guarantees)
            - [Minimal guarantees ("raw storage")](#minimal-guarantees-raw-storage)
            - [Towards better guarantees ("enum with unknown discriminant")](#towards-better-guarantees-enum-with-unknown-discriminant)
            - [Better guarantees](#better-guarantees)
- [Future directions](#future-directions)
    - [Unsafe blocks](#unsafe-blocks)
    - [Other ideas](#other-ideas)
- [Delayed and unresolved questions](#delayed-and-unresolved-questions)
- [Drawbacks (v1.0)](#drawbacks-v10)
- [Alternatives (v1.0)](#alternatives-v10)
- [Edit History](#edit-history)

<!-- /TOC -->

# Summary

Provide native support for C-compatible unions, defined via a new "contextual
keyword" `union`, without breaking any existing code that uses `union` as an
identifier.

# Motivation

Many FFI interfaces include unions.  Rust does not currently have any native
representation for unions, so users of these FFI interfaces must define
multiple structs and transmute between them via `std::mem::transmute`.  The
resulting FFI code must carefully understand platform-specific size and
alignment requirements for structure fields.  Such code has little in common
with how a C client would invoke the same interfaces.

Introducing native syntax for unions makes many FFI interfaces much simpler and
less error-prone to write, simplifying the creation of bindings to native
libraries, and enriching the Rust/Cargo ecosystem.

A native union mechanism would also simplify Rust implementations of
space-efficient or cache-efficient structures relying on value representation,
such as machine-word-sized unions using the least-significant bits of aligned
pointers to distinguish cases.

Drop rules for unions provide enhanced control over destroying values, so unions
can be used for implementing types like `ManuallyDrop` which are useful for
Rust itself and not related to FFI.

# Overview

A union declaration uses the same syntax as a struct declaration, except with
`union` in place of `struct`.

```rust
#[repr(C)]
union MyUnion {
    f1: u32,
    f2: f32,
}
```

The key property of unions is that all fields of a union share common storage.
As a result writes to one field of a union can overwrite its other fields,
and size of a union is determined by the size of its largest field.

A value of a union type can be created using the same syntax that is used for
struct types, except that it must specify exactly one field:

```rust
let u = MyUnion { f1: 1 };
```

The expression above creates a value of type `MyUnion` with
[active field](#active-fragment-and-active-field) `f1`.  
Active field of a union can be accessed using the same syntax as struct fields:

```rust
let f = u.f1;
```

Inactive fields can be accessed as well (using the same syntax) if they are
sufficiently [layout compatible](#minimal-guarantees-raw-storage) with the
current value kept by the union. Reading incompatible fields results in
undefined behavior.  
However, the active field is not generally known statically, so all reads of
union fields have to be placed in `unsafe` blocks.

```rust
unsafe {
    let f = u.f1;
}
```

Writes to union fields may generally require reads (for running destructors),
so these writes have to be placed in `unsafe` blocks too.

```rust
unsafe {
    u.f1 = 2;
}
```

Commonly, code using unions will provide safe wrappers around unsafe
union field accesses.

Another way to access union fields is to use pattern matching.
Pattern matching on union fields uses the same syntax as struct patterns,
except that the pattern must specify exactly one field and `..` is not
supported. Since pattern matching accesses potentially inactive fields it has
to be placed in `unsafe` blocks as well.

```rust
fn f(u: MyUnion) {
    unsafe {
        match u {
            MyUnion { f1: 10 } => { println!("ten"); }
            MyUnion { f2 } => { println!("{}", f2); }
        }
    }
}
```

Pattern matching may match a union as a field of a larger structure. In
particular, when using a Rust union to implement a C tagged union via FFI, this
allows matching on the tag and the corresponding field simultaneously:

```rust
#[repr(u32)]
enum Tag { I, F }

#[repr(C)]
union U {
    i: i32,
    f: f32,
}

#[repr(C)]
struct Value {
    tag: Tag,
    u: U,
}

fn is_zero(v: Value) -> bool {
    unsafe {
        match v {
            Value { tag: I, u: U { i: 0 } } => true,
            Value { tag: F, u: U { f: 0.0 } } => true,
            _ => false,
        }
    }
}
```

Since union fields share common storage, gaining write access to one
field of a union can give write access to all its remaining fields.
Borrow checking rules have to be adjusted to account for this fact.
As a result, if one field of a union is borrowed, all its remaining fields
are borrowed as well for the same lifetime.

```rust
// ERROR: cannot borrow `u` (via `u.f2`) as mutable more than once at a time
fn test() {
    let mut u = MyUnion { f1: 1 };
    unsafe {
        let b1 = &mut u.f1;
                      ---- first mutable borrow occurs here (via `u.f1`)
        let b2 = &mut u.f2;
                      ^^^^ second mutable borrow occurs here (via `u.f2`)
        *b1 = 5;
    }
    - first borrow ends here
    assert_eq!(unsafe { u.f1 }, 5);
}
```

As you could see, in many aspects (except for layouts, safety and ownership)
unions behave exactly like structs, largely as a consequence of inheriting their
syntactic shape from structs.  
This is also true for many unmentioned aspects of Rust language (such as
privacy, name resolution, type inference, generics, trait implementations,
inherent implementations, coherence, pattern checking, etc etc etc).

>Unless some difference is explicitly mentioned in this or the next section,
behavior of a union should coincide with behavior of an equivalent struct.

# Detailed design

## Definitions

### Union variant

Same as union field. These two terms are further used interchangeably.

### Trivially destructible type

Either of the following:

- primitive scalar type (including references) or `str`, or
- aggregate (including arrays and closures) having trivially destructible
component types and not implementing `Drop`, or
- union not implementing `Drop`, or
- anonymous type with `Copy` bound.

If the type is trivially destructible, then dropping its value is a no-op and
doesn't require running any code.

### Structural fragments

Consider a struct or union type having some fields that, in their turn, have
their own nested fields.  
For brevity this example uses anonymous structs and unions which don't currently
exist in the language.

```rust
s: struct {
    a: union {
        b: u8,
        c: struct {
            d: enum {
                A { x: u8 }
            },
            e: u8,
        },
        f: u8,
    },
    g: struct {
        h: &'static u8,
        i: (u8, u8),
    }
}
```

The top level aggregate represents a tree-like hierarchical structure with
primitive indivisible leaf nodes (we consider enums indivisible and ignore
arrays for simplicity). Let's call subtrees of this tree structural fragments
(or, for brevity, simply fragments). Let's also define direct fragments (1 level
of nestedness) and leaf fragments (the indivisible ones).

Borrow checker uses more complex fragments (it recurses into enums and
references), but these simplified fragments are good enough for
move/initialization checking and talking about guarantees for unions.

In the example above `s.a` and `s.g` are direct fragments of `s`.  
`s.a.b`, `s.a.c.d`, `s.a.c.e`, `s.a.f`, `s.g.h`, `s.g.i.0` and `s.g.i.1` are
leaf fragments.

See also [Delayed and unresolved questions](#delayed-and-unresolved-questions).

### Active fragment and active field

Active fragment of a union is its most recently assigned fragment that wasn't
a nested fragment of the previous active fragment.

Active field of a union is its direct active fragment, if it exists.

Example:
```rust
u: union {
    a: struct {
        x: u8,
        y: u8,
    },
    b: u8,
}

u.b = 0; // b is the active fragment and field
u.a.x = 0; // a.x is the active fragment
u.a = 0; // a is the active fragment and field
u.a.y = 0; // a is still the active fragment and field
```

In general case the active fragment/field is not known statically.

See also [Delayed and unresolved questions](#delayed-and-unresolved-questions).

### Initialization state

A fragment can be either in initialized or uninitialized state.
This state is known statically and move checker will allow accesses only to
initialized fragments.
(For simplicity we will consider conditionally initialized fragments with
dynamic drop flags to be uninitialized.)

If fragment has both initialized and uninitialized nested fragments then it's
still uninitialized (we call it partially (un)initialized) and accesses to this
fragment as a whole are prevented by move checker.  
If fragment has only initialized nested fragments then it's initialized as a
whole and can be accessed.  
If fragment has only uninitialized nested fragments then it's is uninitialized
as a whole and cannot be accessed.

A fragment becomes initialized when it's assigned to, or created using
initializer, or it's a union field and its sibling becomes initialized,
or all its nested fragments become initialized.  
A fragment becomes uninitialized when it doesn't implement `Copy` and is moved
out from, or it's a union field (possibly `Copy`) and its sibling becomes
uninitialized, or some of its nested fragments becomes uninitialized.

Example:
```
struct S {
    x: String,
    y: String,
}

let mut s: S; // s, s.x, s.y are uninitialized
s.x = String::new(); // s.x is initialized, s and s.y are uninitialized
s.y = String::new(); // s, s.x, s.y are initialized
let x = s.x; // s.y is initialized, s and s.x are uninitialized
```

NOTE: The "fragment becomes initialized when all its nested fragments become
initialized" rule is not currently implemented and the compiler accepts less
code than it should.

## Union declaration

Unions are declared using the same syntax as structures

```rust
#[repr(C)]
#[derive(Copy, Clone)]
pub union MyUnion<T: Copy> where T: Debug {
    pub f1: u32,
    pub f2: f32,
    f3: T,
}
```

except that tuple unions (`union U(u8, u16);`) and unit unions (`union U;`)
are not permitted.

Empty unions with zero fields (`union U {}`) are permitted and equivalent to
uninhabited empty enums (`enum E {}`) following the
["enum with unknown discriminant"](#towards-better-guarantees-enum-with-unknown-discriminant)
interpretation of unions.

### Contextual keyword

`union` is not a keyword and can be used as identifier in all positions where
identifiers are permitted. For example:

```rust
fn union() {
    let union = 10;
    union;
    union as u8;
}
```

`union` is treated as a start of union declaration only if it's found in item
position (possibly after `pub` and/or attributes) and is followed by a
non-keyword identifier (union name).  
This way it doesn't cause any ambiguities in the Rust grammar and can
be introduced without breaking any existing code that uses `union` as an
identifier.

### Representation attributes

Attributes `#[repr(C)]` and `#[repr(packed)]` are permitted on unions.
Attribute `#[repr(simd)]` is permitted on structs, but not on unions.

Layouts of unions using `#[repr(C)]` are usually defined by ABI documents
provided by C compilers, or hardware vendors, or some standardization
groups.

Typically, alignment of a union with `#[repr(C)]` is equal to the alignment of
its most aligned field, and size of a union is equal to the size of its largest
field rounded up to union alignment (so unions can have trailing padding).  
Note that those maximums may come from different fields; for instance:

```rust
#[repr(C)]
union U {
    f1: u16,
    f2: [u8; 4],
}

fn main() {
    assert_eq!(std::mem::size_of<U>(), 4);
    assert_eq!(std::mem::align_of<U>(), 2);
}
```

Alignment of a union with `#[repr(C, packed)]` is typically equal to 1, and its
size is equal to the size of its largest field (so packed unions do not have
trailing padding).

By default, if `#[repr(C)]` is not used, layout of a union is *unspecified*.

Regardless of representation attributes, all the union fields and the union
itself are located at the same address in memory (pointers to them are equal).  
For `#[repr(C)]` unions this property is guaranteed by ISO C standards.

### Dynamically sized fields

Unions cannot contain fields with dynamically sized types like `[u8]` or
`Trait`.

### Unions and `#[derive]`

Unions support deriving a few traits that do not require accessing their fields.
Supported traits are

- `Copy`, supported only for unions with `Copy` fields
- `Clone`, supported only for unions implementing `Copy`, which can be cloned
trivially
- `Eq` (but not `PartialEq`), supported for unions with `Eq` fields.

Since accessing union fields reliably requires extra knowledge, traits trying to
do it (e.g. `PartialEq`) cannot be derived automatically.

## Constructing union values

A value of a union type can be created using the same syntax that is used for
struct types, except that it must specify exactly one field:

```rust
let u = MyUnion { f1: 1 };
```

This expression will create a value of type `MyUnion` with active field `f1`.  
Union expressions, unlike struct expressions, do not support functional record
update (FRU) `MyUnion { f1: 1, ..v }`.  
Note that creating a union with union expression is safe (no potentially
unreliable data is accessed) and therefore doesn't require `unsafe` block.

Alternatively, a union can be created in uninitialized state and filled in
later.
```rust
let u: MyUnion;
u.f1 = 1; // u becomes initialized, f1 is the active field
          // NB: this is not actually implemented right now, but it should be
```

## Accessing union fields

### Field access (`.`)

Union fields can be accessed using the same dot syntax as struct fields
```rust
unsafe {
    let a = u.f1;
    u.f2 = 1.0;
    fn_call(&u.f2);
}
```

All accesses to union fields (both reads and writes, and also borrows) require
`unsafe` blocks.
The active field/fragment of a union can always be accessed safely, but
reads of inactive fields/fragments can result in undefined behavior (see
[Guarantees and undefined behavior](#guarantees-and-undefinde-behavior) for
more detail).

Write to a union field can potentially overwrite contents of its other fields.  
Write to a union field should not modify any bits outside of the modified field
(except for possibly by running destructor of the old field value).

When a new value is assigned to a union field, the old value of the field is
dropped normally. This matches the behavior of `struct` fields, but differs
from assignments to the whole union, which ignore field destructors by default
(see [Unions and drop](#unions-and-drop) for more detail). Example:

```rust
struct NoisyDrop;
impl Drop for NoisyDrop {
    fn drop(&mut self) {
        println!("BOOM!");
    }
}

union U {
    a: NoisyDrop
}

fn main() {
    unsafe {
        let mut u = U { a: NoisyDrop };
        u.a = NoisyDrop; // BOOM!
        u = U { a: NoisyDrop }; // Silence
    }
}
```

### Pattern matching

Alternatively, union fields can be accessed using pattern matching.

Pattern matching on union fields uses the same syntax as struct patterns,
except that the pattern must specify exactly one field or zero fields and `..`.
Both refutable and irrefutable patterns are supported as usual.

```rust
    unsafe {
        match u {
            MyUnion { f1: PATTERN } => {} // refutable
            MyUnion { f2 } => {} // irrefutable
        }
    }
```

Since union fields are accessed during pattern matching, the code doing it
needs to be placed into `unsafe` blocks. All the guarantees and rules about
undefined behavior that are applicable to dot accesses to fields applicable to
pattern matching accesses as well.  
Pattern matching with zero fields and `..` doesn't require an unsafe block
because it doesn't access any fields.

Attribute `#[structural_match]` cannot be applied to unions, so constants in
patterns cannot contain union values inside them. Matching on union fields is
unsafe and this unsafety should not be hidden in constants.

```rust
const C: U = U { a: 10 };

match C {
    C => {} // ERROR: cannot use unions in constant patterns
    _ => {}
}
```

## Unions in constant expressions

Unions can be constructed in constant expressions and stored in constants.
```rust
const C: MyUnion = MyUnion { f1: 1 };
```

The active field of a union constant is set once during its creation and
cannot be changed later since constant evaluation is pure.
All constant evaluation happens during compilation, so the active fields for
a certain constant union value is always known.

One notable restriction of unions during constant evaluation is that only this
known active field can be accessed. Reinterpreting union's memory as a value of
some inactive field type is not supported at compile time.

## Unions and special traits

### Unions and `Drop`

Structures drop all their fields when destroyed.
Unions don't generally know what fields can be reliably accessed during
destruction, so they cannot do the same thing as structs.

What unions do is completely skipping the field dropping phase and therefore
leaking their contents.
If some cleanup is required, then programmer has to perform it manually,
by implementing the `Drop` trait for the union itself or some larger data
structure including it.

Alternatively, Rust unions could behave like C++ unions and report an error
on destruction if they have non trivially-destructible fields. However, Rust
permits dropping any values unconditionally in generic code. So implementing
this alternative would require introducing a new built-in trait (`Destructible`
or something like this) and using it in all generic code that wants to work
with unions. That would be highly impractical.

Implementing `Drop` for unions themselves is permitted.
`Drop` implementation is called as usual during union destruction, but the
following field dropping phase is still skipped.

As a result of the properties described above, the `needs_drop` intrinsic
returns `false` for unions that don't implement `Drop` themselves.

Unions with non [trivially-destructible](#trivially-destructible-type) fields
are expected to be relatively rare. To avoid surprises with unintentional
leaking of fields, such unions are reported by a special warn-by-default lint.  
The lint's name is `unions_with_drop_fields`, so the warning can be
silenced using `#[allow(unions_with_drop_fields)]` attribute if necessary.  
Lints have to be reported before monomorphization and translation of generics,
so the lint has to work pessimistically and report fields of not yet known
generic types as well (unless they implement `Copy`, then they are guaranteed
to be trivally-destructible). Example:

```rust
// `S` doesn't implement `Drop`, but it's still not trivially-destructible.
struct S(String);

union U<T, U: Copy> {
    a: S, // Reported, not trivially-destructible
    b: T, // Reported, too generic to check before monomorphization.
    c: U, // Not reported, known to implement `Copy`.
}
```

### Unions and `Copy`

`Copy` can be implemented for a union if all its fields are `Copy`.  
This generally falls under "unions behave like structs unless specified
otherwise", however it's important to note that this property is required to
maintain [guarantees](#guarantees) like "accessing the active field is
safe". If these guarantees are abandoned and unions are treated as a piece of
raw storage entirely maintained by programmer, then restrictions on `Copy`
implementations could be lifted as well.

## Ownership

Since union fields share and own common storage, gaining write access to one
field can give write access to all remaining fields of a union.
Borrow and move checking rules have to be adjusted to account for this fact.

### Borrow checking

If struct field is borrowed (mutably or immutably), then the struct as a whole
(and other parent fragments) is borrowed as well (mutably or immutably).  
Note that fragments used for borrowing are a bit more complex than those
described in ["Structural fragments"](#structural-fragments) and include
fragments under references in particular.

If union field is borrowed, the behavior is analogous to a struct fields
(the union as a whole and other parent fragments are borrowed) *plus* all its
sibling fields are borrowed as well.  
So we effectively borrow the piece of storage under the borrowed field
preventing undesirable accesses to it through any possible names.

Example:
```rust
// ERROR: cannot borrow `u` (via `u.f2`) as mutable more than once at a time
fn test() {
    let mut u = MyUnion { f1: 1 };
    unsafe {
        let b1 = &mut u.f1;
                      ---- first mutable borrow occurs here (via `u.f1`)
        let b2 = &mut u.f2;
                      ^^^^ second mutable borrow occurs here (via `u.f2`)
        *b1 = 5;
    }
    - first borrow ends here
    assert_eq!(unsafe { u.f1 }, 5);
}
```

Consider a more complex example with borrowing a field of a struct inside of
a union.

```rust
struct S {
    x: u32,
    y: u32,
}

union U {
    s: S,
    both: u64,
}

fn test() {
    let mut u = U { s: S { x: 1, y: 2 } };
    unsafe {
        let bx = &mut u.s.x;
        // let bboth = &mut u.both; // This would fail
        let by = &mut u.s.y;
        *bx = 5;
        *by = 10;
    }
    assert_eq!(unsafe { u.s.x }, 5);
    assert_eq!(unsafe { u.s.y }, 10);
}
```

`let bx = &mut u.s.x;` needs to borrow the piece of storage under `u.s.x` so
it borrows `u.s.x` itself, then `u.s` as its parent, then `u.both` as a sibling
of `u.s` because `u.s` is a union field, then `u` as a parent of `u.s`.
Note that `u.s.y` stays unborrowed since it doesn't belong to the "piece of
storage under `u.s.x`".  
In other words, simultaneous borrows of multiple fields of a struct contained
within a union do not conflict.

### Move and initialization checking

Move rules for unions follow the same logic as borrowing rules.
We want to control ownership over a piece of storage regardless of names it can
be accessed by.

If struct field [becomes uninitialized](#initialization-state) (e.g. moved
out), then the struct as a whole and other parent fragments become
uninitialized.  
If union field becomes uninitialized, then the union
as a whole and other parent fragments become uninitialized *plus* all its
sibling fields become uninitialized.

If struct field becomes initialized (e.g. assigned to), then the struct as a
whole and other parent fragments may become initialized if this was the only
missing fragment of the struct.  
If union field becomes initialized, then all its sibling fields and therefore
the union as a whole become initialized, some other parent fragments may become
initialized too as a result.

NOTE: The "fragment becomes initialized when all its nested fragments become
initialized" rule is not currently implemented (for both structs and unions)
and the compiler accepts less code than it should.

## Guarantees and undefined behavior

### Requirements and desirable properties

#### Type punning

It's most important to note that unlike unions in C++ and early versions of C,
Rust unions are *intended* to be used for type punning.  
Union fields other than the recently assigned one can be read without UB in
many cases.  
This is important to support existing coding practices. People used and use
type punning with unions in C/C++ despite what the standards say and popular
compilers support this use. People will use type punning with unions in Rust as
well (knowingly or unknowingly) in code related or not related to FFI.  
Rust, unlike C and C++, can easily support this use because it doesn't have to
support type based aliasing rules.

Further we consider several examples of unions used for type punning and
disscuss what should be considered a defined behavior and what should not.

##### Layout compatibility at type level

Let's start with something unambiguous:
```rust
#[repr(C)]
union U {
    float: f32,
    int: u32,
}

let u = U { float: f32 };
let int = u.int;
```

`u32` is layout compatible with `f32` as a type, bit representation of any valid
value of type `f32` is also a bit representation of valid value of type `u32`.  
This should not be UB.

##### Layout compatibility at value level

More ambiguous example.
```rust
#[repr(C)]
union U {
    int: u8,
    boolean: bool,
}

let u1 = U { int: 1 };
let boolean1 = u1.boolean;
let u2 = U { int: 2 };
let boolean2 = u2.boolean;
```

`bool` is not layout compatible with `u8` as a *type*, there are bit
representations of valid values of type `u8` not being bit representation of
any valid value of type `bool`.  
`boolean1 = u1.boolean` still should probably be valid because `bool` is layout
compatible with *value* held in the active fields `u1.int` when the "cast"
happens.  
`boolean2 = u2.boolean` is unambiguosly UB because `0b0000_0010` is not a bit
representation of a valid `bool` value.

##### History of assignments

One more ambiguous example.
```rust
#[repr(C)]
union CPU {
    rax: u64,
    eax: u32,
}

let mut u = CPU { rax: 0xffff_ffff_ffff_ffff };
u.eax = 0x0000_0000;
let rax = u.rax;
assert_eq!(rax, 0xffff_ffff_0000_0000); // assuming little endian
```

`let rax = u.rax;` tries to read the value of inactive field `rax` (`u.eax` was
assigned most recently) and `u64` is not layout compatible with any value of
type `u32` - there are extra bits.  
This still should probably be valid because bit representation of the whole
union `CPU` contains a valid bit representation of `u64` at necessary offset
left from previous assignmensts.  
For the assert to pass it's also required for the assignment `u.eax = ...` to
not overwrite any bits outside of `u.eax`.

##### Reading uninitialized bits

Bad but still not completely unambiguous example.
```rust
#[repr(C)]
union U {
    int: u8,
    nothing: (),
}

let u = U { nothing: () };
let int = u.int;
```

While any bit pattern that can be contained in `u` represents a valid value of
type `u8` this example still should probably be UB because `let int = u.int;`
tries to read bits that were never initialized.

#### Other desirable properties

##### Ownership issues

Consider the next example:
```rust
union U {
    s: String,
}
impl Copy for U {}

let u1 = U { s: String::new("Hello!") };
let u2 = u1;
let s1 = u1.s;
let s2 = u2.s;
let s3 = u2.s;
```

Here we create three different `String`s `s1`, `s2` and `s3` owning the same
dynamically allocated `"Hello!"`. UB happens when we try to free this allocation
more than once.  
Rust generally prevents this kind of UB and it would be desirable to prevent it
for unions as well, and it is indeed statically preventable.
If [move checker rules](#move-and-initialization-checking) are
enforced for unions and unions with non-`Copy` fields
[cannot implement `Copy`](#unions-and-copy), then ownership related issues
will not happen.

##### Reading the active field

One more desirable property is ability to always access the most recently
assigned field safely. Consider this code:

```rust
union U {
    s: String,
}
impl Copy for U {}

let u1 = U { s: String::new("Hello!") };
let u2 = u1;
drop(u2.s);
let s = u1.s;
```

Despite `u1.s` being the active field of `u1` it cannot be accessed safely
and `let s = u1.s;` results in UB because the string was already dropped by
`u2`.  
This kind of UB can be statically prevented as well by the same
[move checker rules](#move-and-initialization-checking) and
[restrictions on `Copy` impls](#unions-and-copy).

### Guarantees

Building on the requirements and desired properties from the previous section
we will now describe what compile-time and run-time guarantees we can and want
to provide for unions.

There are two polar views on how to interpret unions.
The one pole is the "raw storage" interpretation giving less guarantees and more
freedom, and another pole is the "enum with unknown discriminant" interpretation
giving more guarantees at cost of some restrictions.

#### Minimal guarantees ("raw storage")

Union value represents a region of raw storage.  
Programmer is fully responsible for maintaining this storage in usable state.  
Fields are merely a convenient way to reinterpret some parts of that storage
as values of certain types.

Guarantee: Reading a fragment `f` having type `T` from of a union value `u` is
permitted if and only if the bit pattern of `u` contains a valid value of type
`T` starting from `offset(t)` and this value has no uninitialized bits (except
for maybe in padding), otherwise such read results in UB.
(Note that writes may require reads too if destructor is called.)  
Alternatively, even uninitialized bits could be permitted if they still form
a valid value.

NOTE: Borrowing or `memcpy`ing an invalid value still may be considered legal
in some circumstances, this is a problem orthogonal to unions and it's
currently under consideration by
["Unsafe Code Guidelines" team](https://internals.rust-lang.org/t/next-steps-for-unsafe-code-guidelines/3864).

To determine that a valid value of target type exists at certain offset,
a programmer has to know layouts of involved types in detail. As mentioned in
[Representation attributes](#representation-attributes) section, for
`#[repr(C)]` structs and unions such knowledge can be found in third party ABI
documents. "repr(Rust)" structs and unions have unspecified layouts, so their
inactive fields cannot be accessed reliably.

While "raw storage" guarantees satisfy requirements listed in
[Type punning](#type-punning) section, they are too weak to provide
[Other desirable properties](#other-desirable-properties).
Unions fully managed by programmer are susceptible to ownership issues and
cannot guarantee that the recently written field can be read safely.  
If borrow checks are turned off for unions in addition to move checks, then
unions become susceptible to aliasing issues as well.  
Rust is generally known for preventing ownership and aliasing issues statically
and these issues feel sufficiently orthogonal to other kinds of unsafety
specific to unions, like reading an invalid field, so we may want to abandon
the "raw storage" interpretation in favor of some other interpretation with
stronger guarantees.

#### Towards better guarantees ("enum with unknown discriminant")

An attractive way to interpret unions is to treat them as enums with unknown
discriminant having at least one valid variant at any moment of time.

Indeed, a union value is initialized with a valid variant/field when created.
Each time some field is assigned the variant stored in the union can be changed.
If the union value is not yet initialized or, vice versa, already moved out,
move checker can statically ensure that this value cannot be used.  
All guarantees from the "raw storage" interpretation can be kept and inactive
variants still can be accessed if they are layout compatible.

Unfortunately, this interpretation is not entirely adequate because nested
fragments of union variants, unlike fragments of enum variants, are freely
accessible.

Consider the next example:
```rust
#[repr(C)]
union U {
    a: (u8, bool),
    b: (bool, u8),
}

let mut u = U { a: (2, false) }; // union's memory is (2, 0)
u.b.1 = 2; // turns union's memory into (2, 2)
           // (2, 2) is neither valid (u8, bool) nor (bool, u8)
```

Here we turn an initialized union into a state with zero valid variants by
assigning to a fragment of an inactive variant. To fix the problem we need to
think in terms of field fragments instead of fields.

#### Better guarantees

Union is interpreted as an enum with unknown discriminant, but the set of
possible variants is extended with all its nested fragments.

Guarantee 1: All [minimal guarantees](#minimal-guarantees-raw-storage) are kept,
except that the accessed fragment has to be in
[initialized state](#initialization-state), otherwise move checker will report
an error. This means inactive fragments still can be accessed if the union
storage contains an appropriate bit pattern.

Guarantee 2: If the union is in initialized state, then its active fragment
exists and contains a valid value.

"Guarantee 2" can be provided if [borrow checks](#borrow-checking),
[move/initialization checks](#move-and-initialization-checking) and
[restrictions on `Copy` impls](#unions-and-copy) are enabled.

Consequences of "Guarantee 2":
- Unions with a single field are always in valid state. This means wrapper
  types like `NoDrop` are completely safe.
- Unions whose fields have the same type are always in valid state. This
  property also requires the "all fields have the same address"
  rule from [Representation attributes](#representation-attributes) section.
  ```rust
  union U {
      name: String,
      alias_name: String,
  }
  ```
  This guarentee can be used for creating safe "field aliases" with different
  names referring to the same content.

# Future directions

## Unsafe blocks

The number of `unsafe` blocks required to work with unions can be reduced.  
Unsafe blocks are burden and many unsafe blocks required now are innecessary.  
These false positives diminish value of blocks containing something actually
unsafe and requiring attention.

Actions that are safe but currently require unsafe blocks:
- Writes to trivially-destructible fields, unconditionally.  
  Not requiring unsafe blocks for such writes would be useful for both FFI and
  non-FFI code.
- Unions whose fields have the same type, including unions with a single field
  are safe if the "enum with unknown discriminant" interpretation of unions is
  chosen and [Better guarantees](#better-guarantees) are given.  
  Fields of such unions could be read and written without unsafe blocks. This
  will make wrapper unions like `NoDrop` completely safe.

Safe fields.  
Consider a union similar to
[`LARGE_INTEGER`](https://msdn.microsoft.com/en-us/library/windows/desktop/aa383713%28v=vs.85%29.aspx)
from WinAPI.
```rust
#[repr(C)]
union U {
    qword: u64,
    dwords: (u32, u32),
}
```
It's statically known that all accesses to fragments of this unions are safe
(given that [initialization checker](#move-and-initialization-checking) is
enabled) and contain valid values regardless of order of fields assignments,
but the compiler doesn't know about this and unsafe blocks are still required.
It would be useful to communicate this knowledge to compiler by marking these
fields safe.
```rust
#[repr(C)]
union U {
    safe qword: u64, // one possible syntax
    !unsafe dwords: (u32, u32), // alternative possible syntax
}
```

## Other ideas

Pattern matching on multiple fields of a union at once.
```rust
match u {
    U { float, int } => println!("{} {}", float, int);
}
```
For rationale, consider a union using the low bits of an aligned pointer as a
tag; a pattern match may match the tag using one field and a value identified
by that tag using another field.  
Usual borrowing rules have to apply - several fields can be used in a union
pattern only if they are bound immutably or by `Copy` value. Binding one field
mutably will result in borrowing errors for other fields.

C APIs using unions often also make use of anonymous unions and anonymous
structs. For instance, a union may contain anonymous structs to define
non-overlapping fields, and a struct may contain an anonymous union to define
overlapping fields. Such anonymous types could be implemented in Rust as well:
```rust
struct S {
    a: u8,
    b: union {
        c: u8,
        d: i8,
    }
}
```
Declarations like this could create a type with some unique unusable name at
the closest possible scope to the point of declaration.

Recursive unions (https://github.com/rust-lang/rfcs/issues/1804).
```rust
union Foo {
    f0: u8,
    f1: Foo,
}
```
Unlike recursive structs, recursive unions do not require infinite storage.

Dynamically sized fields in unions can be permitted
(https://github.com/rust-lang/rust/issues/36312).
```rust
union Foo {
    f0: u8,
    f1: [u8],
}
```
Such fields will make the union itself dynamically sized.

The idea with separating size and stride and removing trailing padding is
applicable to unions as well as to structs
(https://github.com/rust-lang/rfcs/issues/1397).

Layouts of unions without `#[repr(C)]` (so called "repr(Rust)" unions) can be
specified. There's not much choice in how unions can be layed out in memory.
The remaining uncertainty is mostly about trailing padding and separation of
size and stride, mentioned in the previous paragraph.

Accesses to inactive union fields can be permitted in constant expressions.  
This requires reinterpreting memory at compile time and therefore needs
some amount of target emulation abilities from the compiler
(https://github.com/rust-lang/rust/issues/32836#issuecomment-243855448).
[MIRI](https://github.com/solson/miri) is reported to have such abilities.

# Delayed and unresolved questions

Specify what happens with [fragments](#structural-fragments) implementing
`Drop`. How presence of unions affects initialization rules for structs with
destructor, do they still act as a single indivisible fragment? What about
unions with destructor? How all this affects [guarantees](#guarantees)?

What if some fragment is partially modified without an assignment?
What if an inactive leaf fragment enum is opened and some of its inner parts
are modified? How this affects definition of
[active fragment](#active-fragment-and-active-field) and
[guarantees](#guarantees)?

How do unions interact with enum layout optimizations
(https://github.com/rust-lang/rust/issues/36394)?

# Drawbacks (v1.0)

Adding a new type of data structure would increase the complexity of the
language and the compiler implementation, albeit marginally.  However, this
change seems likely to provide a net reduction in the quantity and complexity
of unsafe code.

# Alternatives (v1.0)

Proposals for unions in Rust have a substantial history, with many variants and
alternatives prior to the syntax proposed here with a `union` pseudo-keyword.
Thanks to many people in the Rust community for helping to refine this RFC.

The most obvious path to introducing unions in Rust would introduce `union` as
a new keyword.  However, any introduction of a new keyword will necessarily
break some code that previously compiled, such as code using the keyword as an
identifier.  Making `union` a keyword in the standard way would break the
substantial volume of existing Rust code using `union` for other purposes,
including [multiple functions in the standard
library](https://doc.rust-lang.org/std/?search=union).  The approach proposed
here, recognizing `union` to introduce a union declaration without prohibiting
`union` as an identifier, provides the most natural declaration syntax and
avoids breaking any existing code.

Proposals for unions in Rust have extensively explored possible variations on
declaration syntax, including longer keywords (`untagged_union`), built-in
syntax macros (`union!`), compound keywords (`unsafe union`), pragmas
(`#[repr(union)] struct`), and combinations of existing keywords (`unsafe
enum`).

In the absence of a new keyword, since unions represent unsafe, untagged sum
types, and enum represents safe, tagged sum types, Rust could base unions on
enum instead.  The [unsafe enum](https://github.com/rust-lang/rfcs/pull/724)
proposal took this approach, introducing unsafe, untagged enums, identified
with `unsafe enum`; further discussion around that proposal led to the
suggestion of extending it with struct-like field access syntax.  Such a
proposal would similarly eliminate explicit use of `std::mem::transmute`, and
avoid the need to handle platform-specific size and alignment requirements for
fields.

The standard pattern-matching syntax of enums would make field accesses
significantly more verbose than struct-like syntax, and in particular would
typically require more code inside unsafe blocks.  Adding struct-like field
access syntax would avoid that; however, pairing an enum-like definition with
struct-like usage seems confusing for developers.  A declaration using `enum`
leads users to expect enum-like syntax; a new construct distinct from both
`enum` and `struct` avoids leading users to expect any particular syntax or
semantics.  Furthermore, developers used to C unions will expect struct-like
field access for unions.

Since this proposal uses struct-like syntax for declaration, initialization,
pattern matching, and field access, the original version of this RFC used a
pragma modifying the `struct` keyword: `#[repr(union)] struct`.  However, while
the proposed unions match struct syntax, they do not share the semantics of
struct; most notably, unions represent a sum type, while structs represent a
product type.  The new construct `union` avoids the semantics attached to
existing keywords.

In the absence of any native support for unions, developers of existing Rust
code have resorted to either complex platform-specific transmute code, or
complex union-definition macros.  In the latter case, such macros make field
accesses and pattern matching look more cumbersome and less structure-like, and
still require detailed platform-specific knowledge of structure layout and
field sizes.  The implementation and use of such macros provides strong
motivation to seek a better solution, and indeed existing writers and users of
such macros have specifically requested native syntax in Rust.

Finally, to call more attention to reads and writes of union fields, field
access could use a new access operator, rather than the same `.` operator used
for struct fields.  This would make union fields more obvious at the time of
access, rather than making them look syntactically identical to struct fields
despite the semantic difference in storage representation.  However, this does
not seem worth the additional syntactic complexity and divergence from other
languages.  Union field accesses already require unsafe blocks, which calls
attention to them.  Calls to unsafe functions use the same syntax as calls to
safe functions.

Much discussion in the [tracking issue for
unions](https://github.com/rust-lang/rust/issues/32836) debated whether
assigning to a union field that implements Drop should drop the previous value
of the field.  This produces potentially surprising behavior if that field
doesn't currently contain a valid value of that type.  However, that behavior
maintains consistency with assignments to struct fields and mutable variables,
which writers of unsafe code must already take into account; the alternative
would add an additional special case for writers of unsafe code.  This does
provide further motivation for the lint for union fields implementing Drop;
code that explicitly overrides that lint will need to take this into account.

# Edit History

- v1.2 This RFC was extended in https://github.com/rust-lang/rfcs/pull/1897.
  - The existing implementation, guarantees and tradeoffs are described in more
    detail.
  - Future directions are outlined.
  - Small tweaks: `#[structural_match]` is prohibited on unions, empty unions
    are permitted, `..` is permitted in union patterns.
- v1.1 This RFC was amended in https://github.com/rust-lang/rfcs/pull/1663/
  - The behavior of individual union fields whose type implements `Drop` is
  clarified.
