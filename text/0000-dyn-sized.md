- Feature Name: `dyn_sized`
- Start Date: 2018-01-25
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a (non-default-bound) marker trait `DynSized` and the corresponding lints to statically prevent
an `extern type` from being used where run-time size and alignment are needed.

# Motivation
[motivation]: #motivation

Foreign type (`extern type`, [RFC #1861]) is equivalent to C’s incomplete struct, often used to
create opaque pointers. These types have no known size and alignment, even at run-time. RFC #1861
noted that `size_of_val` and `align_of_val` should not be defined for `extern type`, but does not
specify how.

> … we must also be careful that `size_of_val` and `align_of_val` do not work either, as there is
> not necessarily a way at run-time to get the size of extern types either. For an initial
> implementation, those methods can just panic, but before this is stabilized there should be some
> trait bound or similar on them that prevents their use statically.

These functions are later implemented to return size of 0 and alignment of 1 as a stopgap solution
in [PR #44295].

`DynSized` was introduced as part of the competing [RFC #1993]. All types except opaque data types
implement `DynSized`, and thus can solve the `size_of_val` problem. Like `Sized`, it is an implied
bound that needs to be opt-out via `?DynSized`, and `T: ?Sized` will still imply `T: DynSized`.

The `DynSized` trait was implemented in [PR #46108]. However, making `DynSized` an implied bound
caused push back from the team. The `?Trait` feature was considered confusing, and causing pressure
and churn to package authors to generalization every `?Sized` to `?DynSized`. Further details can be
found in [RFC issue #2255].

This RFC attempts to find a solution which will

1. not cause breakage within the current epoch,
2. can statically detect misuse of `size_of_val`, and
3. does not require `?DynSized`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In Rust there are types which the compiler does not know the exact size. Slices `[T]` is one
example: the length of a slice is unknown when compiling. These types are known as
*dynamically sized types* (DSTs), as the exact size can only be known at run-time.

There are types which the size *cannot* be known even at run-time. They are mostly used when
interacting with external library, where objects are encapsulated behind opaque pointers. These are
called *foreign types* in Rust, and declared using the syntax `extern { type Foreign; }`.

Some Rust features expect types have a run-time size and thus cannot accept foreign types. An
obvious example is the `std::mem::size_of_val` function which computes the run-time size of a value.
Some less obvious examples are:

* Rust allows DSTs to be used as a struct field:

    ```rust ,ignore
    struct Data {
        checksum: u32,
        data: [u8], // <-- a DST
    }
    ```

    However, we need to know the alignment of the DST field to calculate its offset. A foreign type
    has no run-time alignment, and thus cannot be used inside a struct.

* Allocation of an object obviously requires its size and alignment. Moreover, deallocation also
    needs these information. This makes `Box<Foreign>` invalid.

To prevent us from accidentally allowing a foreign type in where run-time size is needed, the
standard library provides an additional marker trait `DynSized`, to indicate the type has a run-time
size. `DynSized` is automatically implemented for all types *except* foreign types.

We may bound a generic parameter by `DynSized` to totally exclude foreign types.

```rust ,ignore
unsafe fn as_byte_slice<T: DynSized + ?Sized>(val: &T) -> &[u8] {
//                         ^~~~~~~~ we require the type `T` to have run-time size.
    let size = size_of_val(val);
    let ptr = val as *const T as *const u8;
    unsafe { slice::from_raw_parts(ptr, size) }
}

let _ = as_byte_slice(&1);      // ok! sized types implement DynSized.

let _ = as_byte_slice("foo");   // ok! str also implement DynSized.

extern {
    type Foreign;
    fn get_foreign() -> *mut Foreign;
}

let _ = as_byte_slice(&*get_foreign());     // error! extern type does not implement DynSized.
```

Additionally, when we use types which possibly doesn’t have a run-time size in `size_of_val`, `Box`
or a struct field, there will be warnings:

```rust ,ignore
let _: Box<Foreign> = Box::from_raw(get_foreign());
//^ warning! cannot use extern type in a box.

struct Foo<T: ?Sized> {
    a: u8,
    b: T,   // warning! T might not have a run-time size
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `DynSized` trait

Introduce a new marker trait `core::marker::DynSized`.

```rust ,ignore
#[unstable(feature = "dyn_sized", issue = "999999")]
#[lang = "dyn_sized"]
#[rustc_on_unimplemented = "`{Self}` does not have a size known at run-time"]
#[fundamental]
pub trait DynSized {}
```

Modify the `core::marker::Sized` marker trait to:

```rust ,ignore
#[stable(feature = "rust1", since = "1.0.0")]
#[lang = "sized"]
#[rustc_on_unimplemented = "`{Self}` does not have a constant size known at compile-time"]
#[fundamental]
pub trait Sized: DynSized {
    //           ^~~~~~~~ new
}
```

`DynSized` implementations are automatically generated by the compiler. Trying to implement
`DynSized` should emit error [E0322].

`DynSized` will be implemented for the following types:

* `Sized` types i.e.
    * primitives `iN`, `uN`, `fN`, `char`, `bool`,
    * pointers `*const T`, `*mut T`,
    * references `&'a T`, `&'a mut T`,
    * function pointers `fn(T, U) -> V`,
    * arrays `[T; n]`,
    * never type `!`,
    * unit tuple `()`,
    * closures and generators
* slices `[T]`
* string slice `str`
* trait objects `dyn Trait`
* structs, tuples, enums and unions where all fields are `DynSized`

`DynSized` will *not* be implemented for the following types:

* foreign types
* structs, tuples, enums and unions where at least one field is not `DynSized`

`DynSized` is *not* a default bound. When `T: ?Sized`, we do not assume `T: DynSized`. Traits will
not have an implicit `DynSized` super-bound.

## `#[assume_dyn_sized]` attribute

To uphold stability guarantee, we are not going to introduce `DynSized` bound to existing generic
bounds in the standard library.

Instead, we introduce a generic-parameter attribute `#[assume_dyn_sized]`:

```rust ,ignore
fn size_of_val<#[assume_dyn_sized] T: ?Sized>(val: &T) -> usize;
//             ^~~~~~~~~~~~~~~~~~~
```

`#[assume_dyn_sized] T: X` can be thought as `T: DynSized + X`, except that when we substitute a
non-`DynSized` type into `T`, it simply triggers a warning instead of an error.
`#[assume_dyn_sized]` should not affect inference result.

The `#[assume_dyn_sized]` attribute is considered an implementation detail and should not be
stabilized. Use of this attribute outside of the standard library is strongly discouraged, and the
proper bound `DynSized + ?Sized` should be used instead.

`#[assume_dyn_sized] T` does not really mean `T: DynSized`, e.g. the following should cause a
type-check error:

```rust ,ignore
fn foo<T: DynSized + ?Sized>() {
}
fn bar<#[assume_dyn_sized] T: ?Sized>() {
    foo::<T>(); // <-- error: T does not implement DynSized.
}
```

The `#[assume_dyn_sized]` attribute should be added to the following functions and types:

* `align_of_val`, `size_of_val`
* `RefCell`
* `Rc`, `Weak`
* `Arc`, `Weak`
* `Box`
* `Mutex`
* `RwLock`

In rustdoc, render use of this attribute `#[assume_dyn_sized] T: X` as `T: DynSized + X`.

## Lints

It is an error to use to non-`DynSized` types as a struct field or in `size_of_val`/`align_of_val`.
To avoid introducing breaking changes, we are going to close this gap across 3 milestones (one
milestone is one epoch or smaller time units).

### Milestone 0: Warning period

* Move into this milestone after `DynSized` is implemented (but is unstable). Do not stabilize
    `extern type` before this milestone is complete.

If a type cannot prove that it implements `DynSized`, but is used in places where `size_of_val`
etc are needed, a lint (`not_dyn_sized`, warn-by-default) will be issued. The places which triggers
the lint check are:

1. In a struct/tuple field, except the following conditions:

    * The field is the first and only field in the struct, or
    * The struct is `#[repr(packed)]`

2. In an enum variant (if we allow DST enum)
3. A type `T` substituted into a generic parameter which is annotated `#[assume_dyn_sized]`.

The type `T` passes the check when:

1. It can be proved to implement `DynSized`, or
2. It originates from a generic parameter annotated `#[assume_dyn_sized]`, or
3. It is a struct/tuple/union/enum where all fields satisfy one of these 3 conditions.

This lint **must never** be emitted in stable/beta channels in this milestone.

<details><summary>Examples</summary>

Check 1:

```rust ,ignore
struct Foo<T: ?Sized> {
    a: u8,
    b: T,
}
```

```
warning: type `T` is not guaranteed to have a known size and alignment, and may cause panic at run-time
 --> src/foo.rs:3:7
  |
1 | struct Foo<T: ?Sized> {
  |               ------ hint: change to `DynSized + ?Sized`
2 |     a: u8,
3 |     b: T,
  |        ^ alignment maybe undefined
  |
  = note: #[warn(not_dyn_sized)] on by default
  = note: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
```

Check 2:

```rust ,ignore
struct Bar<#[assume_dyn_sized] T: ?Sized> {
    a: Foo<T>, // no lint here!
}
extern { type Opaque; }
let _: Bar<Opaque>;
```

```
warning: type `Opaque` has no known size and alignment, and will cause panic at run-time
 --> src/bar.rs:5:11
  |
5 | let _: Bar<Opaque>
  |            ^^^^^^ size is undefined
  |
  = note: #[warn(not_dyn_sized)] on by default
  = note: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
```

Check 3:

```rust ,ignore
struct A<T: DynSized + ?Sized>(u8, (T,)); // no lint!
struct B<T: ?Sized>(u8, (T,)); // lint!
struct C<#[assume_dyn_sized] T: ?Sized>(u8, (T,)); // no lint!
```

</details>

### Milestone 1: Denial period

* Move into this milestone after the `DynSized` trait is stabilized.

Make `not_dyn_sized` deny-by-default, and enable the lint on stable/beta channel.

### Milestone 2: A new epoch

* Move into this milestone after a new epoch.

Turn the `not_dyn_sized` lint to a hard-error when the new epoch is selected.

### Milestone 3: Proper trait bounds

Replace all `#[assume_dyn_sized]` by proper `T: DynSized` bounds.

Since “breaking changes to the standard library are not possible” using epochs ([RFC #2052]), it may
be impossible to reach this milestone.

## Run-time behavior

Since violation of `#[assume_dyn_sized]` is just a lint, even if we misuse `size_of_val` the code
can still be successfully compiled, and we need to consider their behavior at run-time (i.e. after
monomorphization):

* When a struct/tuple field is a non-`DynSized` type, assume its alignment is 1. This ensures
    accessing a struct field `&foo.bar` will never panic.

* When `size_of_val`/`align_of_val` is instantiated with a non-`DynSized` type, generate a panic
    call. (An alternative is adapt the status-quo of returning 0 and 1 respectively.)

## Survey of existing DST usage

As a sanity check, we want to know how the community uses DST in order to know how the new lint will
affect them. We gather this information by categorizing DST usage of the “[top 100 packages] +
dependencies” (totally 191 packages) provided by Rust playground. We consider a piece of code is
“using DST” whenever `?Sized`, `size_of_val` or `align_of_val` appears.

From the list, many usage patterns are not affected by `DynSized`, since they often just needs to
give a pointer address to method.

<!-- spell-checker:disable -->

* <details><summary>142 packages (≈76%) do not use any of these DST features</summary>

    ```
    adler32
    advapi32-sys
    atty
    backtrace-sys
    bit-set
    bit-vec
    bitflags
    build_const
    cc
    cfg-if
    chrono
    cmake
    color_quant
    cookie
    crc
    crossbeam
    crypt32-sys
    csv-core
    data-encoding
    dbghelp-sys
    debug_unreachable
    deflate
    docopt
    dtoa
    either
    enum_primitive
    env_logger
    extprim
    filetime
    fixedbitset
    flate2
    foreign-types
    foreign-types-shared
    fuchsia-zircon
    fuchsia-zircon-sys
    futf
    futures-cpupool
    gcc
    getopts
    glob
    hpack
    html5ever
    httparse
    hyper-tls
    idna
    image
    inflate
    iovec
    itoa
    jpeg-decoder
    kernel32-sys
    language-tags
    lazy_static
    lazycell
    libflate
    libz-sys
    log
    lzw
    mac
    markup5ever
    matches
    memchr
    memmap
    mime
    mime_guess
    miniz-sys
    native-tls
    num
    num-bigint
    num-complex
    num-integer
    num-iter
    num-rational
    num-traits
    percent-encoding
    phf_codegen
    phf_generator
    pkg-config
    precomputed-hash
    regex-syntax
    relay
    rustc-demangle
    rustc_version
    safemem
    same-file
    scoped-tls
    scoped_threadpool
    scopeguard
    secur32-sys
    select
    semver
    semver-parser
    serde_codegen_internals
    serde_derive
    serde_derive_internals
    siphasher
    slab
    smallvec
    solicit
    string_cache
    string_cache_codegen
    string_cache_shared
    strsim
    synom
    syntex_errors
    syslog
    take
    tempdir
    term
    term_size
    termcolor
    termion
    textwrap
    thread-id
    threadpool
    time
    tokio-core
    tokio-proto
    tokio-tls
    typeable
    unicode-bidi
    unicode-normalization
    unicode-segmentation
    unicode-width
    unicode-xid
    unreachable
    url
    utf-8
    utf8-ranges
    uuid
    vcpkg
    vec_map
    version_check
    void
    walkdir
    winapi
    winapi-build
    winapi-i686-pc-windows-gnu
    winapi-x86_64-pc-windows-gnu
    wincolor
    ws2_32-sys
    xattr
    ```

    </details>

* **Static trait object** —

    ```rust
    fn read_from<R: Read + ?Sized>(r: &mut R) -> Result<Self>;
    //              ^~~~              ^~~~~~
    ```

    A function takes an `&T` or `&mut T` reference, where the type `T` implements a trait. The
    `?Sized` bound allows the function to cover dynamic trait objects since `dyn Trait: Trait`. Its
    usage is typically limited to functions provided by the trait, and seldom needs to know the size
    or alignment.

    <details><summary>This pattern is used in 14 packages.</summary>

    ```
    aho-corasick
    ansi_term
    csv (via serde)
    mio
    nix
    phf_shared
    png
    rayon-core
    reqwest
    serde
    serde_json
    serde_urlencoded (via serde)
    syn
    toml (via serde)
    ```

    </details>

* **AsRef** —

    ```rust
    fn open_file<P: AsRef<Path> + ?Sized>(p: &P) -> Result<Self>;
    //              ^~~~~~~~~~~              ^~
    ```

    A function takes an `&T` reference, where `T` implements `AsRef<X>` meaning the `&T` can be
    converted to an `&X` without allocation. This is typically used to accept various kinds of
    strings which are unsized, thus the `?Sized` bound. The function usually immediately call
    `.as_ref()` to obtain the `&X`, and again seldom access the runtime size or alignment.

    <details><summary>This pattern is used in 10 packages.</summary>

    ```
    aho-corasick
    base64
    clap
    mio
    miow
    quote
    regex
    syntex_pos
    unicase
    xml-rs
    ```

    </details>

* **Delegation** —

    ```rust
    impl<'a, T: Read + ?Sized> Read for &'a mut T { ... }
    //          ^~~~           ^~~~     ^~~~~~~~~
    ```

    A trait is reimplemented for smart pointers implementing the trait. The implementation typically
    just dereference the pointer and forward the method. Thus, the allocation aspect of the smart
    pointers are not touched.

    Delegation targets are seen in various forms:

    * <details><summary><code>&T</code> and <code>&mut T</code>: 10 packages</summary>

        ```
        aho-corasick
        bytes
        futures
        quote
        rand
        rayon
        rustc-serialize
        serde
        tokio-io
        toml
        ```

        </details>
    * <details><summary><code>Box&lt;T&gt;</code>: 8 packages</summary>

        ```
        bytes
        futures
        quote
        rand
        rustc-serialize
        serde
        tokio-io
        tokio-service
        ```

        </details>
    * <details><summary><code>Cow&lt;T&gt;</code>: 4 packages</summary>

        ```
        quote
        rayon
        rustc-serialize
        serde
        ```

        </details>
    * <details><summary><code>Rc&lt;T&gt;</code> and <code>Arc&lt;T&gt;</code>: 2 packages</summary>

        ```
        serde
        tokio-service
        ```

        </details>

* **Extension trait** —

    ```rust
    impl<T: Read + ?Sized> ReadExt for T { ... }
    //      ^~~~           ^~~~~~~     ^
    ```

    A trait is blanket-implemented for an existing trait. The `?Sized` is for completeness, and
    otherwise usually implemented like a typical trait which doesn’t need the size and alignment.

    <details><summary>This pattern is used in 5 packages.</summary>

    ```
    byteorder
    gif
    itertools
    petgraph
    png
    ```

    </details>

* **Using `size_of_val` like C’s `sizeof`** —

    ```rust
    let value: c_int = 1;
    setsockopt(
        sck,
        SOL_SOCKET,
        SO_REUSEPORT,
        &value as *const c_int as *const c_void,
        size_of_val(&value),
    //  ^~~~~~~~~~~~~~~~~~~
    );
    ```

    Most `size_of_val` calls are not used to obtain the runtime size of a DST, but to mimic C’s
    `sizeof` operator on a value, which are sized types. It is usually used in FFI scenario.

    <details><summary>This pattern is used in 10 packages.</summary>

    ```
    backtrace
    error-chain
    libc
    mio
    miow
    net2
    num_cpus
    schannel
    syntex_syntax
    unix_socket
    ```

    </details>

* **Comparison** —

    ```rust
    fn find_first<Q: PartialEq<K> + ?Sized>(&self, key: &Q) -> Option<&K> { ... }
    //               ^~~~~~~~~~~~                       ^~
    ```

    A function takes an `&T` reference which implements `PartialEq<X>`, `PartialOrd<X>`,
    `Borrow<X>`, `Hash` or some similar methods that allows comparing the `&T` with an `&X`. This is
    typically used in data-structure types.

    <details><summary>This pattern is used in 5 packages.</summary>

    ```
    bytes
    hyper
    ordermap
    phf
    serde_json
    ```

    </details>

* Miscellaneous usages for `?Sized` bounds such as,
    * just want to use a `&T` without caring what `T` is (`error-chain`, `serde_json`, `tendril`)
    * use it for `Cow<T>` (`ansi_term`)
    * use it in associated type `type T: ?Sized` in order to be generic in accepting a string or
        byte slice (`ansi_term`, `regex`, `tendril`)

Now some usages which will be affected by `DynSized`.

* **Box** —

    ```rust
    struct P<T: ?Sized>(Box<T>);
    //                  ^~~~~~
    ```

    A type which contains a box of arbitrary unsized type. This is used in:

    * hyper (`PtrMapCell<V>`)
    * syntex_syntax (`P<T>`)
    * thread_local (`TableEntry<T>`)

* **DST struct** —

    ```rust
    struct Spawn<T: ?Sized> {
        id: usize,
        data: LocalMap,
        obj: T,
    //  ^~~~~~
    }
    ```

    This is used in:

    * futures (`Spawn<T>`)
    * tar (`Archive<R>`)

* **Unsafe memory copying** —

    ```rust
    let mut target = vec![0u8; size_of_val(&v)];
    //                         ^~~~~~~~~~~~~~~
    ```

    Using `size_of_val` on a maybe-unsized type to `memcpy` the content somewhere else. This is used
    in:

    * nix (`copy_bytes`)

* **Transmuting** —

    Just tries to inspect the DST detail by transmutation or other unsafe tricks. This is used in:

    * traitobject

<!-- spell-checker:enable -->

Due to Rust’s stability guarantee, all above usage should continue to compile, even if it may panic
at runtime.

As we can see from the above statistics, in the 49 packages using DST features, only 12 packages
really assume `DynSized`, and out of which, 5 packages uses `Box<T>`/`Rc<T>`/`Arc<T>` simply for
delegation. This means it is more popular for `?Sized` to just mean “nothing is assumed”.

# Drawbacks
[drawbacks]: #drawbacks

Foreign type is a very exotic feature, but the `not_dyn_sized` lint applies to a much broader area.
Existing code seldom need to care about foreign types, and thus `DynSized` becomes an annoying
paper-cut for most users.

Even with this RFC, we still need to support `size_of_val`/`align_of_val` for non-`DynSized` input.
If we make `size_of_val` panic, dropping a `Box<Foreign>` will also panic which is undesirable. On
the other hand, if we make `size_of_val` return 0 or some made-up value, the allocator would free
the memory using the wrong size, which at best leaks the memory, and at worst overwrites unrelated
memory and causes undefined behavior.

# Rationale and alternatives
[alternatives]: #alternatives

## Rationales

* `DynSized` in this RFC is an empty marker trait. Previous RFCs suggest this trait to provide
    a `size_of_val`/`align_of_val` method which allows “custom DST”. We believe this is not the
    correct level to place such methods:

    1. These two methods need to be defined even for non-`DynSized` types. To the compiler, putting
        `size_of_val`/`align_of_val` inside `DynSized` is not generic enough. It should be defined
        on a global trait like `Any` minus the `'static` bound.

    2. Like `Sized`, we do not want users to implement `DynSized` themselves. Thus the customized
        of size and alignment should be implemented via a sub-trait of `DynSized`.

    In either case, `DynSized` should be empty even when we have custom DST.

* `DynSized` is automatically implemented by the compiler, but is not an auto trait. It can be made
    into an auto trait:

    ```rust ,ignore
    #[unstable(feature = "dyn_sized", issue = "999999")]
    #[lang = "dyn_sized"]
    #[rustc_on_unimplemented = "`{Self}` does not have a size known at run-time"]
    #[fundamental]
    pub auto trait DynSized {}
    ```

    Since `extern type` will not implement any auto traits, this line is enough to distinguish
    foreign types.

    Whether `DynSized` is an auto trait is an implementation detail, and this RFC does not dictate
    the choice. However, there may be compilation [performance problems] with auto traits.

## `DynSized` as implied bound

This RFC does not make `DynSized` an implied bound, in order to workaround [RFC issue #2255]. This
means existing `?Sized` bounds will continue to accept foreign types as input, and we cannot fix the
bounds of `size_of_val` without breaking backward compatibility. Instead we introduce a lint, and
make use of epochs to eventually turn the lint into an error. Still, the bounds of `size_of_val`
will forever be incorrect to support Epoch 2015.

An alternative is just accept that we want `?DynSized`. As stated in [RFC #1993], making `DynSized`
an implied bound has the following benefits:

* Libraries, including the standard library, can relax `?Sized` to `?DynSized` at their own pace
    without losing backward compatibility.
* The standard library can provide the correct bound for all types without the `#[assume_dyn_sized]`
    hack.
* `?Sized`/`?DynSized` looks better than `(DynSized + ?Sized)`/`?Sized`.

On the other hand, adding new implied bounds is currently blocked by the compiler [issue #21974].
Furthermore, concerns are raised about new implied bounds like `?DynSized` and `?Move`:

* Implied bound is a "negative feature", and it is confusing to reason about.
* Unlike the lints, the user needs to manually evaluate every use of `?Sized` and determine whether
    moving to `?DynSized` is acceptable.

## Replace `#[assume_dyn_sized]` by `DynSized` itself

Instead of introducing a new attribute for linting, we may make `DynSized` itself serve the purpose
for linting. We still restrict the bounds for `size_of_val` and friends:

```rust ,ignore
fn size_of_val<T: DynSized + ?Sized>(val: &T) -> usize;
//                ^~~~~~~~
```

However, when we instantiate `size_of_val::<Foreign>`, instead of a type-check error, we emit a lint
instead. This allows us to describe the parameter using the correct bound without breaking existing
code.

The drawback is that it makes the trait bound concept very unnatural — the `DynSized` bound is
written there, but it cannot be used for inference, and violating the bound just causes a warning
🤔.

## Post-monomorphization error

Instead of introducing `DynSized` or lints, we may simply refuse to compile during monomorphization
of `size_of_val`, `align_of_val` or struct field using a foreign type. This gives an accurate
compile-time error and only affects the rare cases where foreign type is actually misused.

Rust currently has no post-monomorphization lints or type errors (except [E0511] or recursion
overflow). Introducing such errors marks a serious departure of this policy.

## Do nothing

Just make `size_of_val`/`align_of_val` panic or return some sensible value, and hope the user won’t
put a foreign type in them.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should `DynSized` be `#[fundamental]`?

[E0322]: https://doc.rust-lang.org/error-index.html#E0322
[E0511]: https://doc.rust-lang.org/error-index.html#E0511
[RFC #1861]: http://rust-lang.github.io/rfcs/1861-extern-types.html
[RFC #1993]: https://github.com/rust-lang/rfcs/pull/1993
[RFC #2052]: http://rust-lang.github.io/rfcs/2052-epochs.html
[PR #44295]: https://github.com/rust-lang/rust/pull/44295
[PR #46108]: https://github.com/rust-lang/rust/pull/46108
[issue #21974]: https://github.com/rust-lang/rust/issues/21974
[RFC issue #2255]: https://github.com/rust-lang/rfcs/pull/2255
[performance problems]: https://github.com/rust-lang/rfcs/pull/1858#issuecomment-337524343
