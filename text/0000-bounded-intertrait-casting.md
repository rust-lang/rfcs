- bounded_intertrait_casting
- 2025-11-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Safe constant-time and minimal space overhead up and down casting between traits. Bounded by a root "super" trait.

# Motivation
[motivation]: #motivation

Rust's trait objects enable powerful abstraction and dynamic polymorphism, but today the language lacks a safe, principled, and efficient mechanism for converting between related trait objects in non-trivial trait hierarchies. In practice, large Rust codebases routinely define families of interrelated traits where a single concrete type implements multiple traits that conceptually belong to the same behavioral "graph." In these situations, it is natural to want conversions such as:

* converting `&dyn TraitA` to `&dyn TraitB`
* converting up and down within a bounded trait hierarchy
* performing these conversions without `'static` constraints, runtime registries, or bespoke machinery

Today, that is not something Rust can express safely or ergonomically.

Ecosystem solutions exist, but they all share fundamental drawbacks. They rely on global registries, dynamic maps, `TypeId` lookups, or user-maintained metadata. These approaches introduce runtime dependencies, require correct registration discipline, and impose performance and optimization penalties. They are rarely constant-time, often force `'static` lifetimes, interact poorly with generics, and are fragile across crate boundaries. More importantly, they force users to rebuild features that the compiler already knows how to reason about: the trait graph, the set of implementing types, and the layout and identity of trait metadata.

Meanwhile, the compiler already possesses the global knowledge required to solve this problem correctly. After monomorphization, the compiler effectively knows:

* every type implementing a particular root trait
* every trait reachable from that root
* the layout and identity of the corresponding vtables

However, Rust currently lacks a mechanism to safely expose and leverage this information for inter-trait casting.

This RFC proposes a language-level facility for bounded inter-trait casting, rooted at an explicitly declared "super trait." For all types participating in a given hierarchy, the compiler computes global, per-type metadata describing which traits are implemented and how to reach them. This enables:

* constant-time, optimizer-friendly checked casts between trait objects sharing a root supertrait
* no runtime registries, no global maps, no user-maintained state
* cross-crate correctness and stability, driven by the compiler's global view
* full lifetime correctness, rather than `'static`-only casting
* support for generics, multiple supertraits, and complex trait graphs

Conceptually, this capability fills the same niche as `dynamic_cast` in C++ or interface casting in JVM languages, but is designed for Rust's compilation and trait systems. It enables richer trait hierarchies, more flexible dynamic polymorphism, and more expressive API design, while remaining consistent with Rust's zero-cost abstraction principles.

In short: developers already want inter-trait casting, and today's ecosystem solutions prove demand but are fundamentally constrained. This RFC provides a sound, efficient, and language-supported path to make inter-trait casting a first-class capability in Rust.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Example usage:
```rust
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }

// These types and traits can be spread out over multiple crates.
struct S0;
struct S1;
struct S2;
struct S3;
pub trait Trait1: SuperTrait { }
pub trait Trait2: SuperTrait { }
pub trait Trait3: Trait1 + Trait2 { }
pub trait Trait4: SuperTrait { }
pub trait Trait5: Trait4 { }
pub trait Trait6: Trait3 + Trait5 { }

/// A trait that is not part of the trait graph.
/// It can't be cast from or to any trait in the graph.
pub trait IrrelevantTrait { }

impl SuperTrait for S0 { }
impl Trait1 for S0 { }

impl SuperTrait for S1 { }
impl Trait2 for S1 { }

impl SuperTrait for S2 { }
impl Trait1 for S2 { }
impl Trait2 for S2 { }
impl Trait3 for S2 { }

impl SuperTrait for S3 { }
impl Trait1 for S3 { }
impl Trait2 for S3 { }
impl Trait3 for S3 { }
impl Trait4 for S3 { }
impl Trait5 for S3 { }
impl Trait6 for S3 { }

#[test]
fn s0() {
    let s = S0;
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait1>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait2>::cast(&s).map(|r| r as *const _).ok(),
        None
    );
}
#[test]
fn s1() {
    let s = S1;
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait1>::cast(&s).map(|r| r as *const _).ok(),
        None
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait2>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait3>::cast(&s).map(|r| r as *const _).ok(),
        None
    );
}
#[test]
fn s2() {
    let s = S2;
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait1>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait2>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait3>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    let s1 = TraitCast::<dyn SuperTrait, dyn Trait1>::cast(&s).unwrap();
    let s2 = TraitCast::<dyn SuperTrait, dyn Trait2>::cast(&s).unwrap();
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait3>::cast(s1).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait3>::cast(s2).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
}
#[test]
fn s3() {
    let s = S3;
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait1>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait2>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait3>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait4>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait4)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait5>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait5)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait6>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait6)
    );

    let s3 = TraitCast::<dyn SuperTrait, dyn Trait3>::cast(&s).unwrap();
    assert_eq!(
        TraitCast::<dyn SuperTrait, dyn Trait4>::cast(s3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait4)
    );
}
```

An example with multiple super traits:
```rust
pub trait SuperTrait1: TraitMetadataTable<dyn SuperTrait1> { }
pub trait SuperTrait2: TraitMetadataTable<dyn SuperTrait2> { }

pub trait Trait1: SuperTrait1 { }
pub trait Trait2: SuperTrait2 { }
pub trait Trait3: Trait1 + Trait2 { }

pub struct S1;
pub struct S2;
pub struct S3;

impl SuperTrait1 for S1 { }
impl SuperTrait2 for S2 { }
impl SuperTrait1 for S3 { }
impl SuperTrait2 for S3 { }
impl Trait1 for S1 { }
impl Trait2 for S2 { }
impl Trait1 for S3 { }
impl Trait2 for S3 { }
impl Trait3 for S3 { }

// S3 will have *two* trait vtable tables: one for SuperTrait1 and one for SuperTrait2.
// S1 and S2 will have only one trait vtable table.

#[test]
fn s3_multiple_supertraits() {
    let s = S3;
    assert_eq!(
        TraitCast::<dyn SuperTrait1, dyn Trait1>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait2, dyn Trait2>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait1, dyn Trait3>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait2, dyn Trait3>::cast(&s).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );

    // So far, so obvious. But what about this?
    let s1 = TraitCast::<dyn SuperTrait1, dyn Trait1>::cast(&s).unwrap();
    let s2 = TraitCast::<dyn SuperTrait2, dyn Trait2>::cast(&s).unwrap();
    // Typeck failure: Trait1 and Trait2 do not share a common supertrait, so this will
    // have unsatisfiable constraints. The current version of the compiler is able to
    // check this.
    // TraitCast::<dyn SuperTrait1, dyn Trait2>::cast(s1)
    // TraitCast::<dyn SuperTrait2, dyn Trait1>::cast(s2)

    // But we can still do this because we know that Trait3 has a shared supertrait with both
    // Trait1 and Trait2:
    assert_eq!(
        TraitCast::<dyn SuperTrait1, dyn Trait3>::cast(s1).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        TraitCast::<dyn SuperTrait2, dyn Trait3>::cast(s2).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
}
```

An example of a generic supertrait:
```rust
pub trait SuperTrait<T>: TraitMetadataTable<dyn SuperTrait<T>> { }

pub trait Trait1: SuperTrait<u8> { }
pub trait Trait2<T>: SuperTrait<T> { }
pub trait Trait3: Trait1 + Trait2<u16> { }

// Same as the multiple supertrait example, but with a generic supertrait.
// Trait3 has two supertraits: SuperTrait<u8> and SuperTrait<u16>.

/// This will have one super trait, after monomorphization.
pub trait Trait4: Trait1 + Trait2<u8> { }
```

## What about lifetimes?
```rust
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }

pub trait Trait1<'a>: SuperTrait {
    fn f(&self) -> &'a u8;
}
struct S1<'a> { a: &'a u8 }

impl<'a> SuperTrait for S1<'a> { }
impl<'a> Trait1<'a> for S1<'a> {
    fn f(&self) -> &'a u8 { self.a }
}

fn outer<'a1>() -> &'a1 u8 {
    let x = 1; 
    let s = S1 { a: &x, };
    inner::<'a1>(&s) // Woa there! This "safely" escapes the lifetime of s!
}

fn inner<'a, 'b>(s: &dyn SuperTrait + 'a) -> &'b u8 {
    // Without restricting lifetimes, this would succeed, escaping `x` 
    // from the scope of `outer`. 
    TraitCast::<dyn SuperTrait + 'a, dyn Trait1<'b> + 'a>::cast(s).unwrap().f()
}
```

Hence, we require downcast-safety in the trait graph: we must not "erase"
lifetimes when "traveling up" the trait graph.

### Trait Selection

Lifetimes can also affect what traits are actually implemented for a given type.
In other words, `'static` is special. Consider the following:
```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait SubTrait<'a>: SuperTrait { }

struct S0<'a>(PhantomData<&'a ()>);
impl<'a> SuperTrait for S0<'a> { }
impl<'a> SubTrait<'a> for S0<'a> { }

struct S1<'a>(PhantomData<&'a ()>);
impl<'a> SuperTrait for S1<'a> { }
impl<'a> SubTrait<'a> for S1<'static> { }
// Technically, `S1<'static>` implements `for<'a> SubTrait<'a>`,
// ie for all lifetimes.

struct S2<'a>(/*...*/);
impl<'a> SuperTrait for S2<'a> { }
impl<'a> SubTrait<'static> for S2<'a> { }
// Note: `S1<'_>` does not implement `for<'a> SubTrait<'a>` (!= `SubTrait<'static>`).
// Trait generics are invariant, so `'static` can't be "relaxed" to any lifetime
// like, e.g., `&'static u8` can.

macro_rules! cast {
  ($a:lifetime, $b:lifetime, $e:expr) => (
    TraitCast::<dyn SuperTrait, dyn SubTrait<$b>>::cast($e as &(dyn SuperTrait + $b)).ok()
  )
}

#[test]
fn static_s0() {
  const S: S0<'static> = S0(/*...*/);
  assert!(cast!('_, 'static, &S).is_some());
}
#[test]
fn non_static_s0() {
  let s = S0(/*...*/);
  fn inner<'a>(s: &'a S0<'a>) {
    assert!(cast!('_, 'a, s).is_some());
    // Fails due to lifetime erasure: 'static is a longer lifetime than 'a.
    assert!(cast!('_, 'static, s).is_none());
  }
  inner(&s);
}
#[test]
fn static_s1() {
  const S: S1<'static> = S1(/*...*/);
  fn inner<'a>(s: &'static S1<'static>, _: &'a ()) {
    assert!(cast!('_, 'a, s).is_some());
    assert!(cast!('_, 'static, s).is_some());
  }
  inner(&S, &());
  assert!(TraitCast::<dyn SuperTrait, dyn for<'out> SubTrait<'out>>::cast(&S).is_ok());
}
#[test]
fn non_static_s1() {
  let s = S1(/*...*/);
  fn inner<'a>(s: &'a S1<'a>) {
    // `S1<'a>` does not implement `SubTrait<'_>` for any lifetime other
    // than `'static`.
    assert!(cast!('_, 'a, s).is_none());
    assert!(cast!('_, 'static, s).is_none());
  }
  inner(&s);
}
#[test]
fn non_static_s2() {
  let s = S2(/*...*/);
  fn inner<'a>(s: &'a S2<'_>) {
    assert!(cast!('_, 'a, s).is_none());
    // `S2<'a>` implements `SubTrait<'static>` for any lifetime `'a`.
    // This doesn't violate the downcast-safety requirement: `'static` is an 
    // erased lifetime.
    assert!(cast!('_, 'static, s).is_some()); // !
  }
  inner(&s);
}
```
Some of these patterns are odd but are nevertheless technically possible when
unsizing directly from a concrete type.

As an aside, we're considering all bound lifetimes, not just those that appear
in the trait definition:
```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait SubTrait: SuperTrait {
  type Assoc;
}
/// Note: we are thinking about all lifetimes, as if like so:
type T3<'a> = dyn SubTrait<Assoc = &'a u8>;
```

### Multiple lifetimes

With multiple lifetimes, we need to ensure that relationships between lifetimes
are preserved independent of erasure. Consider the following:
```rust
trait SuperTrait<'a, 'b>: TraitMetadataTable<dyn SuperTrait<'a, 'b>> { }
trait SubTrait<'a, 'b>: SuperTrait<'a, 'b> { }

#[derive(Default)]
struct S0<'a, 'b> {
  _m0: PhantomData<&'a ()>,
  _m1: PhantomData<&'b ()>,
}
#[derive(Default)]
struct S1<'a, 'b> {
  _m0: PhantomData<&'a ()>,
  _m1: PhantomData<&'b ()>,
}
impl<'a, 'b> SuperTrait<'a, 'b> for S0<'a, 'b> { }
impl<'a, 'b> SuperTrait<'a, 'b> for S1<'a, 'b> { }
impl<'a, 'b> SubTrait<'a, 'b> for S0<'a, 'b> { }
impl<'a, 'b> SubTrait<'a, 'b> for S1<'a, 'b>
where 'b: 'a,
{ }

macro_rules! cast {
  ($a:lifetime, $b:lifetime, $e:expr) => (
    TraitCast::<dyn SuperTrait<'_, '_>, dyn SubTrait<$a, $b>>::cast($e as &dyn SuperTrait<'_, '_>).ok()
  )
}

#[test]
fn a() {
  fn inner<'a, 'b>(_: &'a (), _: &'b ()) {
    let s = S0::<'a, 'b>::default();
    assert!(cast!('a, 'b, &s).is_some());
    let s = S1::<'a, 'b>::default();
    assert!(cast!('a, 'b, &s).is_none());
  }
  inner(&(), &());
}
#[test]
fn b() {
  fn inner<'a, 'b>(_: &'a (), _: &'b ()) 
    where 'b: 'a,
  {
    let s0 = S0::<'a, 'b>::default();
    assert!(cast!('a, 'b, &s0).is_some());
    assert!(cast!('a, 'a, &s0).is_some()); // via variance of S0
    let s1 = S1::<'a, 'b>::default();
    assert!(cast!('a, 'b, &s1).is_some()); // now we can cast S1 to SubTrait.
    assert!(cast!('a, 'a, &s1).is_some()); // via variance of S1
  }
  inner(&(), &());
}
```

## What about cdylibs?

Consider the following structure:
* `A` cdylib
* `B` cdylib
* `C` shared dylib dep

The core problem stems from separately computed `(SuperTrait, Struct, Trait)`
indices in different global crates; the A/B/C topology is just the smallest
shape that exhibits this. Longer chains and more complex dependency graphs 
behave the same way, so we focus on the minimal example.

Consider this *hypothetical* example:
```rust
#![crate_type = "dylib"]
// C.rs
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }

#[repr(C)]
pub struct FfiObject(Box<dyn SuperTrait>);
impl FfiObject {
  pub fn new(inner: impl SuperTrait) -> Self { Self(Box::new(inner)) } 
}
impl core::ops::Deref for FfiObject {
  type Target = dyn SuperTrait;
  fn deref(&self) -> &Self::Target { &self.0 }
}
impl core::ops::DerefMut for FfiObject {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
```
```rust
// B.rs
#![crate_type = "cdylib"]
extern crate C;
use C::*;

trait BTrait: SuperTrait {
  fn thing_done(&self) -> bool;
  fn do_b_thing(&mut self) -> Result<(), Box<str>>;
}

struct InternalB {
  thing_done: bool,
}
impl SuperTrait for InternalB { }
impl BTrait for InternalB {
  fn thing_done(&self) -> bool {
    self.thing_done
  }
  fn do_b_thing(&mut self) -> Result<(), Box<str>> {
    self.thing_done = true;
    Ok(())
  }
}

#[no_mangle]
extern "C" fn init_obj(obj: *mut MaybeUninit<FfiObject>) {
  obj.as_mut_unchecked().write(FfiObject::new(InternalB { thing_done: false }));
}
#[no_mangle]
extern "C" fn uninit_obj(obj: *mut FfiObject) {
  let Some(obj) = obj.as_mut() else { return; };
  core::ptr::drop_in_place(obj);
}
#[no_mangle]
extern "C" fn do_thing(obj: *mut FfiObject) -> core::ffi::c_int {
  let Some(obj) = obj.as_mut() else { return 0; };
  let Ok(obj) = TraitCast::<dyn SuperTrait, dyn BTrait>::cast(&mut *obj) else { return 0; };
  obj.do_b_thing().is_ok() as _
}
#[no_mangle]
extern "C" fn thing_done(obj: *mut FfiObject) -> core::ffi::c_int {
  let Some(obj) = obj.as_mut() else { return 0; };
  let Ok(obj) = TraitCast::<dyn SuperTrait, dyn BTrait>::cast(&mut *obj) else { return 0; };
  obj.thing_done() as _
}
```
```rust
// A.rs
#![crate_type = "cdylib"]
extern crate C;
use C::*;

trait ATrait: SuperTrait {
  fn thing_done(&self) -> bool;
  fn do_a_thing(&mut self) -> Result<(), Box<str>>;
}

struct InternalA {
  thing_done: bool,
}
impl SuperTrait for InternalA { }
impl ATrait for InternalA {
  fn thing_done(&self) -> bool {
    self.thing_done
  }
  fn do_a_thing(&mut self) -> Result<(), Box<str>> {
    self.thing_done = true;
    Ok(())
  }
}

#[no_mangle]
extern "C" fn init_obj(obj: *mut MaybeUninit<FfiObject>) {
  obj.as_mut_unchecked().write(FfiObject::new(InternalA { thing_done: false }));
}
#[no_mangle]
extern "C" fn uninit_obj(obj: *mut FfiObject) {
  let Some(obj) = obj.as_mut() else { return; };
  core::ptr::drop_in_place(obj);
}
#[no_mangle]
extern "C" fn do_thing(obj: *mut FfiObject) -> core::ffi::c_int {
  let Some(obj) = obj.as_mut() else { return 0; };
  let Ok(obj) = TraitCast::<dyn SuperTrait, dyn ATrait>::cast(&mut *obj) else { return 0; };
  obj.do_a_thing().is_ok() as _
}
#[no_mangle]
extern "C" fn thing_done(obj: *mut FfiObject) -> core::ffi::c_int {
  let Some(obj) = obj.as_mut() else { return 0; };
  let Ok(obj) = TraitCast::<dyn SuperTrait, dyn ATrait>::cast(&mut *obj) else { return 0; };
  obj.thing_done() as _
}
```

Think of `A` and `B` as interfaces. `C` is a shared library that `A` and `B` depend on.

Our final crate is a binary that loads `A` and `B` dynamically and calls their functions. Conceptually,
this could also be, e.g., a C++ binary. I am using Rust instead of C++ because I am lazy.
```rust
//! user.rs

#![crate_type = "bin"]

extern crate C; // only used for the FfiObject type

// Lets assume eg cargo is providing us these deps:
extern crate dlopen;
#[macro_use]
extern crate dlopen_derive;

#[repr(transparent)]
struct FfiObject(ManuallyDrop<C::FfiObject>);

#[derive(WrapperApi)]
struct DynamicallyLoadedObjectInterface {
  init_obj: unsafe extern "C" fn(obj: *mut MaybeUninit<C::FfiObject>),
  uninit_obj: unsafe extern "C" fn(obj: *mut C::FfiObject),
  do_thing: unsafe extern "C" fn(obj: *mut C::FfiObject) -> core::ffi::c_int,
  thing_done: unsafe extern "C" fn(obj: *mut C::FfiObject) -> core::ffi::c_int,
}
impl DynamicallyLoadedObjectInterface {
  fn new_obj(&self) -> FfiObject {
    let mut obj = MaybeUninit::uninit();
    unsafe {
      (self.init_obj)(&mut obj);
      FfiObject(ManuallyDrop::new(obj.assume_init()))
    }
  }
  
  fn drop_obj(&self, mut obj: FfiObject) {
    unsafe { (self.uninit_obj)(&mut obj.0) }
  }
}

struct DynamicallyLoadedObject<'r>(&'r DynamicallyLoadedObjectInterface, FfiObject);
impl DynamicallyLoadedObject<'_> {
  fn new(interface: &'r DynamicallyLoadedObjectInterface) -> Self {
    let obj = MaybeUninit::uninit();
    unsafe {
      (interface.init_obj)(&mut obj);
      Self(interface, FfiObject(ManuallyDrop::new(obj.assume_init())))
    }
  }
  fn do_thing(&mut self) -> bool {
    unsafe { (self.0.do_thing)(&mut (self.1).0) != 0 }
  }
  fn thing_done(&self) -> bool {
    unsafe { (self.0.thing_done)(&(self.1).0) != 0 }
  }
}
impl Drop for DynamicallyLoadedObject<'_> {
  fn drop(&mut self) { unsafe { (self.0.uninit_obj)(&mut (self.1).0) } }
}

fn main() {
  let a = unsafe {
    dlopen::load("libA.so")
      .unwrap()
  };
  let b = unsafe {
    dlopen::load("libB.so")
      .unwrap()
  };
  {
    let mut a = DynamicallyLoadedObject::new(&a);
    let mut b = DynamicallyLoadedObject::new(&b);

    assert!(a.do_thing());
    assert!(a.thing_done());
    assert!(b.do_thing());
    assert!(b.thing_done());
    // So far, so good: there aren't any issues as we haven't crossed
    // tables and indices yet.
  }
  // This is where we run into trouble if we tried to share
  // metadata tables across global crates without a "same global-crate" check.
  // The design below prevents this by requiring the global-crate-id equality
  // check.
  
  let mut a_obj = a.new_obj();
  let mut b_obj = b.new_obj();
  
  // Next, we call the libA function with the object created from libB and
  // vice versa.
  // What we'd expect to happen is that the casts would fail,
  // since we created objects that don't implement the other's trait,
  // no harm, no foul. However, the index chosen for ATrait in A’s graph is 
  // likely to coincide with the index chosen for BTrait in B’s graph, which
  // means that the casts will succeed.
  // What happens after is anyone's guess.
  
  // With the cross-global-crate checks, we are able to detect this and prevent
  // the cast from succeeding regardless of potential index overlap.
  assert!(unsafe { (a.do_thing)(&mut b_obj) } == 0);
  assert!(unsafe { (b.do_thing)(&mut a_obj) } == 0);
  
  a.drop_obj(a_obj);
  b.drop_obj(b_obj);
}
```

In full generality, forcing `C` to be the global crate, isn't workable either,
even if all traits are defined in `C`:

> The shape of the metadata slice for `SuperTrait<...>` depends only on the set 
> of cast-target-traits in the graph, which is *not* fully known in C.

But the graph is over lazily monomorphized trait-object types, i.e., nodes like:

* `dyn SuperTrait<u8>`
* `dyn Trait1`
* `dyn Trait2<u16>`
* `dyn Trait2<Downstream>`, etc.

And the castability properties do depend on concrete instantiations:

* `dyn Trait2<u8>` ↔ `dyn SuperTrait<u8>` is a different node/edge than
* `dyn Trait2<Downstream>` ↔ `dyn SuperTrait<Downstream>`.

Crucially, `dyn Trait2<Downstream>` doesn't even exist from C's POV — it only appears once `Downstream` is monomorphized in A.

So if we try to have C define the "canonical index schema" for the graph, we get an impossible requirement:

* C would have to pre-assign indices for *all* future instantiations of generic subtraits over *all* future types that downstream crates might invent.

That's unbounded and unknowable at C's compile time. We can play games like "index by definition, not instantiation", but as soon as:

* our cast semantics distinguish `Trait2<u8>` from `Trait2<Downstream>`, and
* our tables are sized to the reachable monomorphized subtraits for all `(root, downcast target)`s,

we've lost the ability to precompute that shape in C.

Dynamic registries are also out:
* the trait graph determines what traits need to be monomorphized into
  concrete vtables for each type.
* but the trait graph is lazy: only traits that appear as a target of a cast
  are included.

Thus, the loader/dynamic registry will need to know how to codegen vtables for
foreign types when an existing trait is added to the graph via a new downcast
target.

Essentially, you'd need a significant chunk of the Rust compiler and Rust crate
metadata available at runtime.

As a result, we allow multiple global crates at runtime, but we reject casts
across global crate boundaries, even when:
* the root trait is defined in a shared crate (like `C` above),
* the object layout is the same (i.e., is the same concrete type compiled into both artifacts), and
* the traits on both sides are literally the same definition.

A better solution without such drawbacks is, bar a large infra shift, out of reach.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Definitions

Supertrait: ```trait Subtrait where Self: Supertrait {}``` only. Does not include blanket traits over `T: Supertrait`.

Root supertrait: the minimum/top supertrait that a type must implement to be considered a valid instance of a trait graph.
In all the examples in this RFC, `SuperTrait` is the root supertrait.

### Global crate

The potentially-user-selected crate that represents the point at which we can
assume type system information is maximal: there are no downstream or "sibling" 
crates that could add new traits or monomorphizations of upstream traits to
the trait graph. It is a compile time boolean property.

Note: the trait graph is *lazy*: only traits that appear as a target of a cast
are included.

Assumed Global Crates:
* Rust binaries,
* `staticlibs` or `cdylibs` embedded in, e.g. C++, binaries.

Note that dylibs nor rlibs are considered global crates.

Must be overridable at compile time.

Such crates are given a unique identifier, in the form of a unique address,
which is used to identify the trait metadata tables and indices used by that crate.

Note: this is not nessicarily normative, and this proposal is not prescribing 
a specific strategy. The above rules ensure that the metadata
tables and indices are present for linking purposes for existing code/crates, 
even if the casts fail even when, *in theory*, they could succeed with a 
"better global crate choice/etc."

For example, Rust codegen crates are *"dylibs"* and not *"cdylibs"*, but are 
loaded via dlopen with a large amount of host-process shared code; they are
effectively used like a defer-load library: they are known to bootstrap and
could, *in theory*, be "compiled in" w.r.t. downcasting/etc. The 
Rust compiler itself could make use of this proposal, but only with 
other changes that are out-of-scope for this RFC. Since extending the compiler
to support these casts and making use of casts inside the compiler itself
are two separate steps, this RFC does not propose any changes to the Rust 
codegen ecosystem and thus won't affect compatibility with external codegen
crates.

If this definition feels dubious to you, dear reader, then "good": plugin 
architectures are awful and counterproductive to AoT optimizations.


## TraitMetadataTable
[trait-metadata-table]: ##trait-metadata-table

```rust
/// Since this value can only be known globally, the table is computed only for
/// the global crate.
/// It will be implemented for all types and traits that implement/inherit from `SuperTrait`.
/// `SuperTrait` must be a trait object, i.e., `dyn Trait`; `[_]`/`str`/etc is not allowed.
/// "Partially auto" to allow otherwise cyclic trait objectification (via the 
/// `SuperTrait` param, which is actually just a marker).
/// Effectively #[rustc_deny_explicit_impl] due to the impl below.
#[lang_item = "trait_metadata_table"] 
pub trait TraitMetadataTable<SuperTrait>: MetaSized 
  where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>>,
{
    /// Retrieval should /really/ be via a "virtual const" and not a virtual function call.
    /// The returned slice is a static array of all trait vtables for this concrete type. 
    /// The order of the array is implementation defined and subject to whim, but will be the 
    /// same for a given `SuperTrait`.
    /// Effectively a wrapper around `core::intrinsics::trait_metadata_table::<SuperTrait, Self>()`.
    /// Must not dereference any part of `self`.
    fn derived_metadata_table(&self) -> (&'static u8, &'static [Option<NonNull<()>>]);
}

/// Implementation for all types that implement the root `SuperTrait`, and that trait
/// only. Sub-traits won't implement `TraitMetadataTable<SubTrait>`, they will instead
/// implement `TraitMetadataTable<SuperTrait>`.
impl<'a, SuperTrait, T> TraitMetadataTable<SuperTrait + 'static> for T 
    where T: Sized + Unsize<SuperTrait> + 'a,
          SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait + 'static> + 'a,
{
    fn derived_metadata_table(&self) -> (&'static u8, &'static [Option<NonNull<()>>]) {
        core::intrinsics::trait_metadata_table::<SuperTrait, T>()
    }
}

/// Retrieve the index of `Trait`'s vtable in the slice returned via "TraitMetadataTable::derived_metadata_table".
/// The specific index value returned is implementation defined and subject to whim.
/// The value returned is constant for a given `Trait` and `SuperTrait`, but will not be "known
/// enough" to be `const fn` due to the need for a global computation.
/// Note: this value can only be computed globally, i.e., over all crates in the binary.
/// The `&'static u8` is a unique address per global crate only. It is independent of
/// the `SuperTrait` and `Trait` generic params.
#[rustc_intrinsic]
pub fn trait_metadata_index<SuperTrait, Trait>() -> (&'static u8, usize)
    where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>,
          Trait: MetaSized + Pointee<Metadata = DynMetadata<Trait>> + TraitMetadataTable<SuperTrait>;

/// Retrieve the slice returned via "TraitMetadataTable::derived_metadata_table" for the given `SuperTrait`.
/// Calling this intrinsic forces the caller to be delayed until after global monomorphization.
/// The value returned is constant for a given `ConcreteType` and `SuperTrait`, but will not be "known
/// enough" to be `const fn` due to the need for a global computation.
/// Note: this value can only be computed globally, i.e., over all crates in the binary.
/// The `&'static u8` is a unique address per global crate only. It is independent of
/// the `SuperTrait` and `Trait` generic params.
#[rustc_intrinsic]
pub fn trait_metadata_table<SuperTrait, ConcreteType>() -> (&'static u8, &'static [Option<NonNull<()>>])
    where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>,
          ConcreteType: Sized + TraitMetadataTable<SuperTrait>;

/// Return true iff the cast from `SourceTrait` to `TargetTrait` is safe due to lifetime erasure.
/// This is a compile-time check that can be used to ensure that the lifetimes of the source and 
/// target trait object are compatible. Obligation checks are separated from the metadata table entries
/// to facilitate lifetime binders. Usage relaxes the lifetime erasure rules by allowing the source 
/// trait lifetimes to participate in the legality of the cast.
#[rustc_intrinsic]
pub const fn trait_cast_is_lifetime_erasure_safe<SuperTrait, SourceTrait, TargetTrait>() -> bool
    where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>,
          SourceTrait: MetaSized + TraitMetadataTable<SuperTrait>,
          TargetTrait: MetaSized + Pointee<Metadata = DynMetadata<TargetTrait>> + TraitMetadataTable<SuperTrait>;
```

### Significance of the returned `&'static u8` references

A token used to verify metadata tables and table entry indices are from the same
global crate. The returned reference address must be unique per global crate
only. The value of the dereferenced `u8` is unspecified.

LTO must not remove the unique address property.

## TraitCast
[trait-cast]: #trait-cast

```rust
use core::ptr::{Pointee, DynMetadata};
use core::marker::{MetaSized, PointeeSized};

/// In `core`.
#[derive(Debug, Clone, Copy)]
pub enum TraitCastError<T> {
  /// This object is from a different global crate than the one
  /// that is performing the cast.
  /// Useful if you'd like to provide a more informative error message.
  /// Note: do not rely on this behavior. It is subject to change.
  ForeignTraitGraph(T),
  /// This object does not implement the specified trait, or the cast does not
  /// satisfy lifetime erasure requirements. 
  UnsatisfiedObligation(T),
}
impl<T> TraitCastError<T> {
  /// Unwrap the contained, un-casted, value.
  pub fn unwrap(self) -> T {
    match self {
      Self::ForeignTraitGraph(v) | Self::UnsatisfiedObligation(v) => v,
    }
  }
}

/// `I` is the root supertrait.
/// In a future extension, the root supertrait could be implied. Regardless of the specific root supertrait the result of
/// the cast is the same, since the output vtable will be the same after monomorphization
/// (or is essentially user-invisible).
pub trait TraitCast<I: MetaSized, U: MetaSized>: Sized
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          U: Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I>,
{
    type Source: MetaSized + TraitMetadataTable<I>;
    type Target;
    /// Attempt to cast `self` to `U`. All layout and trait satisfaction obligations are enforced,
    /// but lifetime-erasure soundness is not.
    unsafe fn unchecked_cast(self) -> Result<Self::Target, TraitCastError<Self>>;
    /// Attempt to cast `self` to `U`, returning an error if the cast is not 
    /// possible due to lifetime erasure requirements.
    fn checked_cast(self) -> Result<Self::Target, TraitCastError<Self>> {
        if !core::intrinsics::trait_cast_is_lifetime_erasure_safe::<I, Self::Source, U>() {
            return Err(TraitCastError::UnsatisfiedObligation(self));
        }
        unsafe { self.unchecked_cast() }
    }
    fn cast(self) -> Result<Self::Target, Self> {
        self.checked_cast().map_err(TraitCastError::unwrap)
    }
}
impl<'r, T, U, I> TraitCast<I, U> for &'r T
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I> + 'r,
          T: MetaSized + TraitMetadataTable<I>,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'r,
{
    type Source = T;
    type Target = &'r U;
    unsafe fn unchecked_cast(self) -> Result<&'r U, TraitCastError<Self>> {
        unsafe {
            let (obj_graph_id, table) = <T as TraitMetadataTable<I>>::derived_metadata_table(self);
            let (crate_graph_id, idx) = core::intrinsics::trait_metadata_index::<I, U>();
            if crate_graph_id as *const u8 != obj_graph_id as *const u8 {
                return Err(TraitCastError::ForeignTraitGraph(self));
            }
            
            let (p, _) = (self as *const T).to_raw_parts();
            let Some(Some(vtable)) = table.get(idx) else {
                return Err(TraitCastError::UnsatisfiedObligation(self));
            };
            Ok(&*core::ptr::from_raw_parts(p, vtable.get()))
        }
    }
}

impl<'r, T, U, I> TraitCast<I, U> for &'r mut T
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I> + 'r,
          T: MetaSized + TraitMetadataTable<I>,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'r,
{
    type Source = T;
    type Target = &'r mut U;
    unsafe fn unchecked_cast(self) -> Result<&'r mut U, TraitCastError<Self>> {
        unsafe {
            let (obj_graph_id, table) = <T as TraitMetadataTable<I>>::derived_metadata_table(self);
            let (crate_graph_id, idx) = core::intrinsics::trait_metadata_index::<I, U>();
            if crate_graph_id as *const u8 != obj_graph_id as *const u8 {
                return Err(TraitCastError::ForeignTraitGraph(self));
            }
    
            let (p, _) = (self as *mut T).to_raw_parts();
            let Some(Some(vtable)) = table.get(idx) else {
                return Err(TraitCastError::UnsatisfiedObligation(self));
            };
            Ok(&mut *core::ptr::from_raw_parts_mut(p, vtable.get()))
        }
    }
}
/// As written, this is UB due to the formation of the reference needed to get the table pointer.
/// If we could embed constants (relocations, rather) into our vtable, this could be UB-free by loading the table
/// pointer directly from the vtable. For now, this author will ignore this.
impl<T, U, I> TraitCast<I, U> for *const T
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          T: MetaSized + TraitMetadataTable<I>,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I>,
{
    type Source = T;
    type Target = *const U;
    unsafe fn unchecked_cast(self) -> Result<*const U, TraitCastError<Self>> {
        unsafe {
            let (obj_graph_id, table) = <T as TraitMetadataTable<I>>::derived_metadata_table(self.as_ref_unchecked());
            let (crate_graph_id, idx) = core::intrinsics::trait_metadata_index::<I, U>();
            if crate_graph_id as *const u8 != obj_graph_id as *const u8 {
                return Err(TraitCastError::ForeignTraitGraph(self));
            }
            
            let (p, _) = self.to_raw_parts();
            let Some(Some(vtable)) = table.get(idx) else {
                return Err(TraitCastError::UnsatisfiedObligation(self));
            };
            Ok(core::ptr::from_raw_parts(p, vtable.get()))
        }
    }
}
// And so on for Unique, NonNull, and *mut.

/// In `alloc`
impl<'a, T, U, I, A> TraitCast<I, U> for Box<T, A> 
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          T: MetaSized + TraitMetadataTable<I> + 'a,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'a,
          A: Allocator,
{
    type Source = T;
    type Target = Box<U, A>;
    unsafe fn unchecked_cast(self) -> Result<Box<U, A>, TraitCastError<Self>> {
        unsafe {
            let (obj_graph_id, table) = <T as TraitMetadataTable<I>>::derived_metadata_table(&*self);
            let (this, alloc) = Box::into_raw_with_allocator(self);
            let (crate_graph_id, idx) = core::intrinsics::trait_metadata_index::<I, U>();
            if crate_graph_id as *const u8 != obj_graph_id as *const u8 {
                return Err(TraitCastError::ForeignTraitGraph(Box::from_raw_with_allocator(this, alloc)));
            }
            let (p, _) = (this as *const T).to_raw_parts();
            let Some(Some(vtable)) = table.get(idx) else {
                let this = Box::from_raw_with_allocator(this, alloc);
                return Err(TraitCastError::UnsatisfiedObligation(this));
            };
            let p = core::ptr::from_raw_parts(p, vtable.get());
            Ok(Box::from_raw_with_allocator(p, alloc))
        }
    }
}
// And so on for Rc and Arc
```

## Lifetime Erasure or Downcast-Safety

Downcasting via `TraitCast` must not be able to manufacture references whose lifetimes
are longer than those of the underlying concrete value. Informally: after you erase 
some part of a type's lifetime structure, you may not 
reintroduce a "larger" lifetime when casting down.

The unsound pattern this would permit is:

* Start from a trait object `&dyn SuperTrait` whose vtable was produced from some concrete type `C<'a, ...>`.
* Erase the lifetime parameters of `C` at the supertrait boundary.
* Later, cast that same object to a trait `dyn SubTrait<'b, ...>` and treat it as if the underlying `C<'b, ...>` existed, even when `'b` is not compatible with the original `'a`.

To rule this out, we restrict which trait graphs can participate in `TraitCast` and how erased parameters are tracked:

1. **Region closure of subtraits by the root supertrait**

   For a root supertrait `I` and any subtrait `J` that may appear in `I`'s metadata table,
   every lifetime parameter that can appear in the public interface of `J` (method 
   signatures, associated types, supertrait constraints) must be expressible in terms of
   the lifetime parameters of `I`.

   Concretely, there must exist a mapping from `J`'s region parameters to `I`'s region
   parameters such that, for all legal instantiations, the regions used by `J` do not 
   outlive those used by `I`. Intuitively: the root supertrait's lifetimes form a "closure"
   that bounds all lifetimes flowing through any trait reachable from it, so that erasing
   down to `I` does not lose information necessary to check subtrait lifetime soundness.
   
   This implies, for example, you cannot have a non-generic root:

    ```rust
    pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
    pub trait Trait1<'a>: SuperTrait { ... }
    ```
   participate in a downcast-safe graph, because `SuperTrait` has no region parameters that could bound the `'a` of `Trait1<'a>`.

2. **Erased type parameters carry contravariant lifetime information**

   When a trait in the graph erases a type parameter or lifetime parameter along a path
   from `J` to `I`, the compiler conceptually associates that erased parameter with a 
   hidden "erasure lifetime" that is contravariant:

   - The erasure lifetime represents an upper bound on how long the erased component may be assumed to live.

   - Downcasts are only permitted when they decrease this bound (i.e., do not treat an erased component as if it lived longer than it actually does).

   In other words, erased parameters are treated as if they carried a hidden region
   parameter that is contravariant in subtyping, and the trait solver enforces that
   any `TraitCast` instantiation respects these variance constraints. This prevents
   `TraitCast` from turning a short-lived instantiation of a type parameter into a
   long-lived one solely by going "up and then down" the trait graph.

Together, these restrictions ensure that lifetime erasure at the root supertrait is
monotone: any downcast that type-checks cannot extend the lifetimes of the underlying
concrete value or of any references reachable through that value.


## `trait_cast_is_lifetime_erasure_safe`

The `trait_cast_is_lifetime_erasure_safe` intrinsic is used to check whether a
cast from `SourceTrait` to `TargetTrait` is safe due to lifetime erasure. This
check is separated from the metadata table entries to facilitate lifetime
binders.

The computation proceeds as follows:
1. Visit and collect all traits and params from `SourceTrait` and `TargetTrait` to `SuperTrait`.
2. Eliminate the shared prefix from the trait lists. This will compute the gcd 
   (or lcd, depending on how you look at the trait graph) of the trait lists: 
   the `SourceTrait` to `SuperTrait` path will become the "upcast" portion, and
   the `TargetTrait` to `SuperTrait` path will become the "downcast" portion.

## Metadata Table

### Table Entries

Each position in the metadata table corresponds to a concrete instantiation of a
trait in the graph, expanded into multiple entries via lifetime relationships:
* For each trait, there is an extra entry for each unique lifetime relationship
  graph found over all participating type impls.

We need to expand each trait into multiple entries because lifetime
relationships are impl-selection predicates and can be different for different
impls of the trait (ie may be different for each type)

For example:
```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait Trait1<'a, 'b>: SuperTrait { }

struct S1<'a, 'b> {
  // ...
}
impl<'a, 'b> SuperTrait for S1<'a, 'b> { }
impl<'a, 'b> SubTrait<'a, 'b> for S1<'a, 'b> 
  where 'b: 'a,
{ }
struct S2<'a, 'b> {
  // ...
}
impl<'a, 'b> SuperTrait for S2<'a, 'b> { }
impl<'a, 'b> SubTrait<'a, 'b> for S2<'a, 'b>
{ }

// The SuperTrait metadata table layout will need to have three entries:
// 1. The vtable for `SuperTrait`
// 2. The vtable for `Trait1<'a, 'b>`
// 3. The vtable for `Trait1<'a, 'b> where 'b: 'a`
```

### Computation

This section describes how the metadata tables are computed and how the cast function is able to
ensure that the cast is allowed/legal (excluding lifetime erasure).

Note: the specific layout/order is *implementation defined* and subject to whim. In fact,
the table order could be randomly permuted to prevent accidental dependencies.

(Draft note: this is mostly just a rough sketch of the algorithm)

Modify mono:
* In each MIR body: collect contained normalized but not erased
  unique (`SuperTrait`, `Trait`) pairs from the `trait_metadata_index` intrinsic. Collect similar
  unique (`SuperTrait`, `Struct`) pairs from the `trait_metadata_table` intrinsic.
* Any direct call to `trait_metadata_index` / `trait_metadata_table` is treated
  as a monomorphization request that is always fulfilled in the global crate.
  Upstream crates never codegen these intrinsics; they only record them as
  requirements in metadata. Note that `trait_cast_is_lifetime_erasure_safe` is
  not included here.
* Ensure the linkage and visibility of direct references from ^ is linkable downstream.
* In the global crate (i.e., binary or user-designated) only: 
  - form a cardesian product of (`SuperTrait`, `Struct`, `Trait`) tuples,
  - use this set to compute the metadata tables and indices via a spanning tree algorithm,
  - for each `Struct`, prune the unsatisfiable `Trait`s (involves trait solver),
  - prune `Trait`s that no `Struct` can satisfy, change those indices to be isize::MAX.
  - codegen delayed MIR and required vtables with the aforementioned tables and indices.

Note that traits that violate the lifetime erasure constraints *are* present in
the tables. The `trait_cast_is_lifetime_erasure_safe` intrinsic is used to guard
against these violations, with an unsafe escape hatch for lifetime binder 
implementations.

This will result in a few empty/`None` (`SuperTrait`, `Struct`, `Trait`) entries, representing
downwards casts that are not satisfiable. These are unavoidable, since the metadata tables must
be uniform over all `Struct`s for each `SuperTrait`. However, at runtime trait satisfaction is
a fast single branch on null.

## Codegen

Codegen crates themselves should need no change.

## Diagnostics

TODO

# Drawbacks
[drawbacks]: #drawbacks

There are a lot of moving parts here due to the need to bridge between lifetime erasure,
monomorphization, and cross-crate boundaries.

Code size impact is minimal:
* casting: we reduce runtime computation down to two loads, an integer add+mul, and two branches.
* additional vtables: we use monomorphization to only include vtables for 
  concrete types and traits that actually participate in downcasting. As a 
  result, unreferenced blanket generic impls are not included.

Data-size impact is also minimal:
* metadata tables: we use monomorphization to only include vtables for concrete
  types and traits that actually participate in downcasting: unreferenced 
  blanket generic impls are not included.

An option to reduce data-size impact would be to shrink the table entry from a `Option<NonNull<()>>`
to an e.g., `Option<NonMaxU32>`, where all vtables are continuous from an e.g., 32-bit base. This
would halve the size of the tables.

This proposal effectively encourages "god" root supertraits, which is perhaps undesirable to some.
The author considers this to be a downside of free-will and not worth arguing over.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Existing solutions to this problem are:
- `intercast` crate: `dyn Trait` to `dyn Trait` casting. Uses a global hashmap to store the trait vtables. Casting is not constant-time and requires virtual dispatch.
- `traitcast` crate: requires AoT knowledge of the trait graph and a runtime type/trait registry. Casting is not constant-time and requires virtual dispatch.

Under the hood, all these crates use `std::any::Any`/`TypeId`: to cast a trait object to another trait object, a two-step process is followed:
- First, the trait object is cast to a raw pointer of the concrete type.
- Then, the raw pointer is cast to the desired trait object type. Rustc attaches the vtable of the desired trait object type to the raw pointer.

However, this approach has a few drawbacks:
* it is not a constant time,
* pessimizes the optimizer due to global lookups and virtual dispatch,
* forces `'static` lifetimes due to `std::any::Any`, and 
* it doesn't work w/ generic traits/types, without also manually monomorphizing the traits/types.

There is another process that is possible, but I don't think is implemented in a general crate: use `rustc_public`
to expose the trait implementations and types. But that wouldn't allow delayed codegen on its own. It would require
multiple complete compilations of the crates: first to extract the trait vtables then a second compilation that could use
the built vtable tables. It would not work cross-crate without additional hacks.

## Dynamically loaded trait graphs

As stated in the guide, this proposal does not support dynamic trait graphs.

## Lifetime Erasure Avoidance by Casting Directly from `SubTrait1` to `SubTrait2`

Lifetime Erasure rules are defined only for the `SuperTrait` to 
`SubTrait1`/`SubTrait2` path, essentially making all casts downcasts. We have to
do this since table entry obligations are not checkable per-type, only
per-trait-object (i.e., once, i.e., w.r.t. the root supertrait).

The alternative would be to add an expensive check per cast: each cast would 
need to compare a compiler-generated, encoded, lifetime relationship graph of
the lifetimes of the source trait and target trait. The latter of which would 
have to live in the metadata table entries. At minimum, this would require an
extra memcmp, and in full generality, it is equivalent to the graph isomorphism 
problem.

# Prior art
[prior-art]: #prior-art

- `dynamic_cast` in C++

Key differences:
- We don't need to patch up data pointers to handle diamond inheritance.
- I am intentionally disregarding dynamically loaded trait implementations, so no runtime graph traversal needed.

Conceptually, C++ could implement casting similarly to this proposal if those two features weren't required.

- Java and C#: interfaces

These are roughly the same ideas. I will also ignore java's array casting, as Rust doesn't have `dyn [Trait]`, 
at least until fat pointers are generalized.

Java assigns each concrete class a vtable for ordinary virtual dispatch and an
independent per-interface dispatch structure ("itable") for every interface that
the class implements. An itable is conceptually a dense, per-interface method
table that the JVM installs into the object's header via an indirection stored
in the class metadata, allowing constant-time resolution of interface calls 
without requiring graph traversal or RTTI lookups. During class loading, the JVM
computes these itables globally: it walks the full interface inheritance graph, 
flattens inherited interface methods into a canonical ordering, and records, for
each concrete class, the implementing method entry corresponding to each 
interface slot. Failed interface casts are handled by consulting this same global
metadata; the checked-cast operation performs a membership test against the 
precomputed interface implementation sets rather than performing structural 
probing at runtime. The net effect is that Java achieves stable, constant-time 
interface dispatch and constant-time checked interface casting at the cost of 
global computation and additional per-class metadata, which is broadly analogous
in spirit to this proposal's globally computed trait-metadata tables and indices.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

## Dyn upcasting

TODO this already works in Rust, talk about merging these two features to eliminate the need for embedded vtable pointers?

## Downcasting to concrete types

As is, this proposal requires Pointee's with specific Metadata types, which preclude concrete types.

However, the proposed lifetime erasure rules could allow a path to safely downcast to a concrete type.

## Can we generalize the global visits?

Generally, we are performing global visits of two things:

- The trait graph rooted at a trait.
- The concrete types implementing the trait (or a trait).

And then we generate additional code and data as a result of those visits. The core capability is to delay until after
global monomorphization, while still allowing typeck/etc to work locally.
