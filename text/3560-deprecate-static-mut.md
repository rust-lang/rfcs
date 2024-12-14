- Feature `static_mut_2024`
- Start Date: 2024-01-26
- RFC PR: [rust-lang/rfcs#3560](https://github.com/rust-lang/rfcs/pull/3560)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Deprecate `static mut` for the 2024 edition of Rust, directing users to switch to interior mutability. (This is not pertinent to `&'static mut`)

# Motivation
[motivation]: #motivation

The existing `static mut` feature is difficult to use correctly (it's trivial to obtain aliasing exclusive references or encounter UB due to unsynchronised accesses to variables declared with `static mut`) and is becoming redundant due to the expansion of the interior mutability ecosystem which easily replaces `static mut`'s functionality.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`static mut` is meant to provide statics that the program can modify after setting their initial value; variables declared with `static mut` can prove quite problematic when used, however:
```rust
static mut X: i32 = 0;

fn main() {
  let a = unsafe { &mut X };
  let b = unsafe { &mut X };

  println!("{a} {b}");
}
```
Recall Rust's borrowing rules: 
- At any given time, you can have either one mutable reference or any number of immutable references to a place.
- References must always be valid.

We simultaneously have two exclusive (mutable) references to the same datum and actively use them in an entirely overlapping fashion, which means we've violated the first rule. This violation means that our code's behaviour is undefined, and the optimiser is free to do with it as it wishes, potentially breaking it. The code is not guaranteed to print "0 0" and may perform something arbitrary.
`static mut` also allows for unsynchronised accesses across multiple threads, which can cause data races, which are also undefined behaviour.
```rust
use std::thread::spawn;

static mut X: usize = 0;

const N: usize = 16;

fn main() {
    let mut thread_pool = Vec::with_capacity(N);
    
    for i in 0..N {
        thread_pool.push(spawn(move || {
            unsafe {
                X = i;
            }
            println!("{}", unsafe { FOO });
        }));
    }

    for thread in thread_pool {
        thread.join().unwrap();
    }
}
```

Here, since the X is not an atomic (with predictable and defined relative ordering) nor synchronised with a `Mutex` or `RwLock`, a data race takes place, printing numbers between 0 and 16 in a vaguely increasing fashion, data races are undefined behaviour and mean that our code is not correct. This and the previous example show that UB is almost trivial to cause with `static mut`, making it prone to occur by accident in a large codebase.

Let's try to use `static mut` for FFI purposes (a typical application of it); this is usually achieved in this fashion:
```rust
// using a symbol exported by C code
extern "C" { static mut _c_symbol: Ty; }

// exporting a symbol from rust code for use by C code
#[no_mangle]
pub static mut _rust_symbol: Ty = val;
```

The use of `static mut` in this code puts our code at risk of causing UB on access, as we saw before. Accesses to `static mut` can become difficult to track and reason about very quickly as the size of the codebase increases. As such, by the 2024 edition, we get a deprecation warning (or even deny-by-default lint):
```rust
// WARNING: `static mut` syntax is deprecated as of the 2024 edition. Consider using std::cell::SyncUnsafeCell<T> or another interior mutability type instead. 
// Read more at (somewhere, maybe rust blog post).
// Note/fix: 
// - extern "C" { static mut _c_symbol: Ty; }
// + extern "C" { static _c_symbol: std::cell::SyncUnsafeCell<Ty>; }
extern "C" { static mut _c_symbol: Ty; }

// WARNING: `static mut` syntax is deprecated as of the 2024 edition. Consider using std::cell::SyncUnsafeCell<T> or another interior mutability type instead.
//  Read more at (somewhere, maybe rust blog post).
// Note/fix: 
// - pub static mut _rust_symbol: Ty = val;
// + pub static _rust_symbol: std::cell::SyncUnsafeCell<Ty> = std::cell:SyncUnsafeCell::new(val);
#[no_mangle]
pub static mut _rust_symbol: Ty = val;
```

Migration from `static mut` in favour of `SyncUnsafeCell` or another alternative makes code easier to audit, as some operations previously unsafe to perform on `static mut` (such as obtaining a raw pointer to the static) become safe, shifting focus entirely to the areas where problems might arise (where the raw pointers are dereferenced) as it is at those points where we create references from raw pointers or use the raw pointers to access the underlying data. Keep in mind, however, that while `SyncUnsafeCell `is a less obvious type/technique to find (harder for beginners to fall into using) and a more verbose one to use, it is still highly unsafe and still does allow someone determined to create aliasing exclusive references to a place; caution should be taken by users of `SyncUnsafeCell` and `UnsafeCell` in general. 

If we follow the diagnostics provided by the compiler, we can migrate our code to a safer version of itself and make it easier to audit for any mistakes by better isolating where they can occur. Using intermediate raw pointers to obtain references also produces marginally better output from the [Miri tool](https://github.com/rust-lang/miri), allowing for more effective automated detection of problems in the code.

In light of [#114447](https://github.com/rust-lang/rust/issues/114447) paving the way to make references to `static mut` disallowed (and obtaining raw pointers potentially even safe) deprecation of the feature may seem like a waste as obtaining an exclusive borrow of a `static _: SyncUnsafeCell<T>` and a `static mut _: T` both become similar processes. 
```rust
// Where A is static SyncUnsafeCell<T> and B is a static mut T
let a = unsafe { &mut *A.get() };
let b = unsafe { &mut *addr_of_mut!(B)};
```

Additionally, some may not be keen on unsafe interior mutability on `static` as a replacement of `static mut`(mutating through a shared reference with runtime checks or programmer responsibility), citing that `mut` on `static mut` declarations should be a sufficient marker that it is the programmer's responsibility to uphold aliasing and validity invariants.

There does not have to be any inherent danger in `static mut` for it to be deprecated in favour of more friendly and effective methods to create a mutable global. Still, there are issues with `static mut`. The first issue is that it's a beginner trap; there is symmetry in the way that Rust currently lets you declare variables:
```rust
// In terms of *surface-level* semantics

// Immutable local
let
// Mutable local
let mut

// Immutable runtime global
static

// Mutable runtime global
static mut
```
This symmetry is a common beginner trap, as it's familiar on two levels:
- Rust introduced `let` and `let mut`, so `static` and `static mut` should work similarly, the only change being `'static` lifetimes on the latter two.
- Mutable globals never hurt anybody in the other languages they've used.
At this stage, many beginners also convince themselves they've reached the point where they need to use unsafe and produce questionable code. 

Disallowing references to `static mut` is a good measure and does eliminate the symmetry of the declarations' behaviour, but it does not eliminate the visual symmetry that takes many beginners to `static mut`. 

Additionally, the fully enabled and referenceless forms of `static mut` violate the principle of least surprise. Rust does not formally require the principle of least surprise, but adhering to it significantly improves the beginner experience. Having only one way to do any given thing makes it easy to comply with the principle.

In light of the previous point, the interior mutability types, in combination with ordinary `static` declarations, are a solid replacement as they offer a great deal of granularity and are a complete toolset concerning control of access to a place; they are not only a comprehensive replacement for `static mut` of all forms but easy to learn and grapple with as they represent applying the same principle with different requirements and invariants.

It has to be made clear to people who try to use `static mut` through the deprecation message that unsafe interior mutability primitives are just a way to match the behaviour of `static mut` closely and that they are not the ideal solution. Instead, they should use the type with the most checked invariants, and that applies to their use case.

Things to (please) note:
- Interior mutability types ***are*** present in `core`. This change does not irreversibly break `no_std` code; `std` reexports the types from `core`.
- `std::cell::SyncUnsafeCell<T>` ***is*** the standard type that behaves most equivalently to `static mut`, but it is not necessarily the type you want:
	- `std::cell::SyncUnsafeCell<T>` ***is***  `repr(transparent)`, which means it has the same layout as the wrapped type within; this is the main reason the type exists.
	- `std::cell::SyncUnsafeCell<T>` ***does not*** have any more overhead than `std::cell::UnsafeCell<T>`, which is nominally zero. This also means that it has no greater overhead than `static mut` by extension.
	- `std::cell::SyncUnsafeCell<T>` ***is*** bound on `T: Sync`, but creating a custom type can alleviate any problems with needing to put `!Sync` types in a `static`. One can make it into `std` (through `core`) at some point.
	- `std::cell::SyncUnsafeCell<T>` (like `std::cell::UnsafeCell<T>`) ***does not*** provide any runtime safety, which one should opt for whenever possible.
- `static mut` ***is not*** the only way to initialise globals without a `const` value; `std::sync::LazyLock<T,_>` (unstable) and `std::sync::OnceLock<T>` provide initialisation on first access and initialisation once, respectively.
- Unsafe code ***should***(probably) be avoided or at least put you on guard.
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
A custom lint will have to be introduced under the `deprecated` family to support this. This can be changed later to reflect the stance of the Rust project on `static mut`.

There is little to no use of `static mut` in the compiler; it is present mainly in `std` and in the implementation of the runtime in a fashion similar to the above code examples; the few points at which `static mut` is used can be migrated to `SyncUnsafeCell` without causing too much ado.

A quick code search seemingly shows thousands of usages of `pub static mut`, and, at first glance, this gives the impression of mass breakage, but it is crucial to consider the following:
1. Whether or not the declared variable is reachable from the crate root.
2. Whether or not it is actual API. (Using mutable statics as API surface is present in C, but likely is highly discouraged and rare in Rust).

In any case, the use of old edition `static mut` variables can be allowed for backwards compatibility.
# Drawbacks
[drawbacks]: #drawbacks
Verbosity increases as values must be wrapped in an interior mutability type to be placed in statics if they're to be modified later, and methods must be used to get at the underlying data. 

There is a hurdle to migrating to the 2027 edition (deny-by-default lints don't count as breakage, so 2024 is free to have a warn-by-default or deny-by-default lint for deprecation), `cargo fix` can potentially perform some fixes, but the effort of implementing `cargo fix` for this has not yet been gauged.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
- Do nothing.

Doing nothing would result in a redundant, jarring feature that serves no purpose other than being a potential new user trap.

# Prior art
[prior-art]: #prior-art
## Work/discussion directly relevant
- Consider deprecation of UB-happy static mut at [#53649](https://github.com/rust-lang/rust/issues/53639)
- Disallow *references* to `static mut` [Edition Idea] at [#114447](https://github.com/rust-lang/rust/issues/114447)
- Deprecate static mut (in the next edition?) on [IRLO](https://internals.rust-lang.org/t/deprecate-static-mut-in-the-next-edition/19975/12)

## Notable, not directly relevant
- `SyncUnsafeCell` at [#95439](https://github.com/rust-lang/rust/issues/95439)
- `LazyCell/Lock` at [#109736](https://github.com/rust-lang/rust/issues/109736)
- `OnceCell/Lock` at [#74465](https://github.com/rust-lang/rust/issues/74465)
- `AssertThreadSafe`at [T-libs#322](https://github.com/rust-lang/libs-team/issues/322) (IMPORTANT)

Many have tried to remove/deprecate `static mut` before; the feature is now sufficiently redundant and 
subject to replacement to put forth a plan for its eventual removal.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should the deprecation lint be warn-by-default or deny-by-default?
- Should we have `cargo fix` support for migration? It's technically possible, but reasonably high effort.

# Future possibilities
[future-possibilities]: #future-possibilities
Removing `static mut` declarations altogether (edition gated) can be done in the future. Using `static mut` from previous editions can be allowed, and a `legacy_static_mut` attribute can be used to allow new declarations to enable code to upgrade without changing the semantics of their declarations if they have a reason not to replace them.
