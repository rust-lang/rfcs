- Feature Name: `fallible_allocation`
- Start Date: 2021-06-11
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Place functionality in `alloc` and `std` which may call a global OOM handler under a feature `infallible_allocation` which is default enabled.
`std` with `infallible_allocation` enabled will depend on `alloc` with `infallible_allocation` enabled.

This proposal specifically does *not* require the creation of any new APIs.
Having these features will likely lead to the creation of new fallible APIs in the future, but those APIs can be discussed and reviewed individually.

# Motivation
[motivation]: #motivation

There are several programming environments where dynamic allocation is considered acceptable, but failure is expected to be handled.

Concrete examples include:

* [Linux](https://lore.kernel.org/lkml/CAHk-=wh_sNLoz84AUUzuqXEsYH35u=8HV3vK-jbRbJ_B-JjGrg@mail.gmail.com/) and other OS Kernel environments.
* Mid-range embedded environments (e.g. CPU firmware, bootloaders, etc.)
* High-reliability processes (e.g. init)
* Services which allocate in response to untrusted user input

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When a developer is writing code in an environment where `abort()`ing (or calling a dedicated OOM handler) is not an appropriate strategy for dealing with allocation failure scenarios, they should consider disabling the `infallible_allocation` feature.
If the developer is writing in a hosted environment, they will disable the feature on the `std` crate.
If the developer is writing in a bare-metal environment, they will instead disable the feature on the `alloc` crate.
This should be presented as an exceptional situation - the vast majority of developers will want the default here.

This will remove a large number of functions - many standard library functions implicitly allocate and do not handle OOM scenarios.

Note also that this will either require the developer to download an additional sysroot via `rustup` or perform a local build, depending on the developer's target.

## Kernel Example
For example, those writing a kernel likely do not want to `abort()` just because memory allocation failed.
Therefore, it would be preferable to them to have the compiler reject

```
fn do_thing(x: usize) {
  other_fn(Box::new(x))
}
```

and would prefer they write

```
fn do_thing(x: usize) -> Result<(), AllocErr> {
  Box::try_new(x).map(other_fn)
}
```

(or more likely, project the `AllocErr` into a local error type).

To prevent the first example from compiling, the developer would add to `Cargo.toml`'s dependencies section:

```
alloc = { default-features = false }
```

If the developer is using other default features of `alloc`, they may need to add those explicitly.
After this, the first example will fail to compile, but the second will succeed.

## Hosted Example
If the developer is authoring a high-reliability process that absolutely cannot crash (e.g. `init`), they will want the hosted equivalent.
For the same code examples in the Kernel section above, they would instead add

```
std = { default-features = false }
```

Adding any additional features they wish to depend on explicitly.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Moving infallible allocation behind a feature
All functions in `alloc` directly depending on the global OOM handler will be gated on the crate feature `infallible_allocation`.
Any functions directly calling allocation APIs in either `alloc` or `std` will either propagate that error to its caller in a handleable way or be moved behind the crate feature `infallible_allocation`.

## Suppressing annotations in `rustdoc`
Functions behind `infallible_allocation` should not have a corresponding `#[doc(cfg())]` attribute.

## Testing
In CI, `std` and `alloc`'s ability to build without the `infallible_allocation` feature only needs to be tested on a single platform (likely `x86_64-unknown-linux-gnu`) in order to prevent developers accidentally forgetting to tag new functions with the appropriate feature gate.

When cutting release, all Tier-2 platforms should perform a build with `infallible_allocation` disabled to ensure nothing architecture-specific slipped through (though this should be rare).

## Making this feature accessible to Cargo
If sysroot crates (`std`, `core`, `compiler_builtins`, `alloc`) are specified as a dependency in the manifest, Cargo will attempt to locate a variant of that crate which has the minimum set of requested features to build the package.
Since features union as usual, having a crate with dependencies declared this way would be similar to a modern day `#![no_std]` crate: crates depending on it could still use the full `std` as usual with no issues.

In an ideal world, this would be driven by the `-Z build-std` feature, but the rest of this feature doesn't need to be blocked on that.
While we should make these features accessible via Cargo, most environments which would consume this feature don't currently use Cargo for their build process for other reasons, so we would still get benefits without it.

### `-Z build-std`

In a world where `build-std` has become default, this feature would be available in the natural way: selecting features on an explicit dependency on `alloc` or `std` during build.

### Alternate Prebuilts

If `build-std` is cancelled or has too long a time horizon, we can consider sysroots containing multiple copies of these libraries instead.
Cargo will attempt to locate a sysroot library with the correct feature set available, depending on an external mechanism (whether `rustup`, the compiler build, or manual developer action) to have placed the alternate libraries in the sysroot.

If Cargo is passed the `-Z allow-extra-sysroot-features` flag, it will be permitted to select variants of sysroot crates with a superset of required features enabled.
This will allow crates to build even in environments which do not have the alternate libraries installed.

# Drawbacks
[drawbacks]: #drawbacks

- Adds annotations on a large number of functions.
  There's no notion of a feature that is "required by default" for all items, so even though `infallible_allocation` may be the common case, it will still be the annotated case rather than the other way around.
  This can be partially mitigated by using feature annotations on entire modules until those modules contain some fallible allocation supporting functions.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternatives
When considering the alternatives, it is important to remember a few things:

* This is not a one-off issue - we have already added `panic_immediate_abort`, and we will likely need to do something similar for `nofp` on platforms which technically support it in the near future.
* Many of the environments which want to handle OOM care deeply about overhead.
* There is an important distinction between providing APIs which make it *possible* to write OOM-safe code and providing APIs which allow build-time checks that the code *is* OOM-safe.

### Unwinding OOM
Transform OOM into a `panic!`-like event which can be unwound up the stack, calling appropriate drops and releasing memory as it unwinds.
This would be paired with a `std::panic::catch_unwind` equivalent for catching OOMs.
Assuming that a `panic=unwind` runtime is in use, this can be implemented using `alloc::set_alloc_error_hook` outside the standard library using a thread-local `Cell<Option<AllocError>>`.
The library would install an OOM hook which when triggered, wrote to the thread-local cell and `panic!`'d.
`catch_alloc_error` could then be implemented in terms of `catch_unwind`, checking the thread-local cell, and re-propagating the error if the cell is not populated.
We could also provide this functionality in the standard library.

#### Issues
- Requires `panic=unwind` as a prerequisite for handling OOMs.
  Frequently, `panic=abort` is chosen in resource-restricted scenarios where handling OOMs is more important.
- Depends on programmer placement of `catch_alloc_error` barriers for OOM safety.
- This is currently blocked on figuring out how to remove `#[rustc_allocator_nounwind]` from `alloc::handle_alloc_error` without [blowing up binary sizes](https://github.com/rust-lang/rust/issues/42808).

### Split crates rather than adding a feature
Rather than adding a feature to control the availability of these functions, create a `fallible_alloc` crate that contains only those functions which handle OOM or cannot OOM.
`alloc` would depend on `fallible_alloc` and re-export it in its entirety.

In this model we would likely not want to split `std` (see the subsequent alternative for details), preventing authoring robust hosted programs.
If we chose to do this regardless, there would be a `fallible_alloc_std` that links against `fallible_alloc`, and a `std` that links against `alloc`.

User code would select the correct crate to link against in order to write their code, likely using crate renaming for ergonomics.

#### Issues
- If we do not split `std`, Rust will remain inappropriate for high-reliability hosted processes.
- If we do split `std`, we run into composability issues.
  When further customizations of `std` become necessary which would remove functions (e.g. removing floating point on a platform that supports it), it will multiply the number of crates, not add to it.
  In the short term, we would end up with `fallible_alloc_nofp_std`, `fallible_alloc_std`, `nofp_std`, and `std`, with the problem only getting worse from there.
  If we split only `alloc`, I do not anticipate similar issues due to the narrower scope of `alloc`, but it is not impossible.
- Users need to write extra boilerplate in code, not just the build system when choosing to use this.

### Only add the feature to `alloc`, not `std`
As a more conservative approach, we could apply this feature (or crate split) to `alloc` *only*.
`alloc` is where the majority of urgent use cases lie, and the number of functions needing annotation is much lower.

#### Issues
- This does not address the high-reliability userspace process use case at all.
- A `std` with some pieces removed compared to a usual build will likely return in other forms in the future, so we might as well use `fallible_allocation` as a pilot to figure out how we want to do it.

## Impact of not doing this
### Potential ecosystem forking in the embedded/kernel space
Embedded and kernel developers are used to having custom `libc` builds, custom toolchains, and other weird quirks to their build processes.
If we do not provide them the tools they need to succeed, then they will make the changes themselves.
We've mostly dodged that on this issue for now via the `no_global_oom_handler` cfg option, but this is not currently a stable or documented interface.
One of the primary purposes of promoting this to a proper feature is to expose this to users long-term so they do not fork or modify to achieve it.

### Long term proliferation of user required `cfg` flags like `no_global_oom_handler`
Traditionally, `cfg` flags are crate-internal, frequently being set by `build.rs` in response to features or autodetection.
Placing features which a *user* may require behind a `cfg` flag in our standard libraries sets a poor precedent when compared to the unionable feature flags.
It would be easy to end up with multiple features for sysroot crates which are mutually contradictory due to using `cfg` tunables instead of features.

Additionally, this sets a bad example for ecosystem crates.
If ecosystem crates look at sysroot provided code and see the use of `cfg` to guard features, they are more likely to do the same.

### Cargo Exclusion
While many embedded/kernel projects will choose to use their own build system for a variety of reasons, supporting this use case has been a [goal](https://www.ncameron.org/blog/cargos-next-few-years/) of Cargo for a while.
Setting `rustflags` is (intentionally) only possible through a Cargo config, not a `Cargo.toml`, so setting the existing `cfg` is beyond the reach of someone building with Cargo.

# Prior art
[prior-art]: #prior-art

## [`no_global_oom_handler`](https://github.com/rust-lang/rust/pull/84266)
The `no_global_oom_handler` cfg flag already implements this division for the `alloc` crate, but intentionally does so in a way that is only accessible at the `rustc` level in order to avoid stabilization.

A major goal of this proposal is to stabilize and expose this separation.

## C++ OOM Handling
In C++, allocation failure has two models - the default `new`, which will throw a `std::bad_alloc` exception, and an overload which additionally takes a `std::nothrow_t` and will produce a `nullptr` in the event of allocation rather than throwing.
Unfortunately, only one of these models is actually used throughout the rest of the STL.
For example, `std::vector::push_back` has no `std::nothrow_t` variant and will throw on allocation failure.
While the working group [explored](http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2018/p0132r1.html) adding such variants, they never came to fruition.

However, many environments intentionally run with exceptions disabled.
The Google Style Guide explicitly [forbids](https://google.github.io/styleguide/cppguide.html#Exceptions) exceptions with the reasoning that it makes programs harder to write (an exception returned in a setting with non-RAII cleanup will cause issues), debug (log messages and traces can come from and go unexpected places), and incurs performance costs.
The Mozilla Style Guide also [forbids](https://firefox-source-docs.mozilla.org/code-quality/coding-style/using_cxx_in_firefox_code.html#c-language-features) the use of exceptions.
While it does not go into detail on the rationale, anecdotes from mailing lists and blogs imply that performance concerns are a primary driver.
Apple [disables](https://developer.apple.com/library/archive/documentation/DeviceDrivers/Conceptual/IOKitFundamentals/Features/Features.html) exceptions in the Darwin kernel, though like Mozilla, they do not detail their reasoning.
In embedded circles, common wisdom is that C++ exceptions and associated unwinding add too much overhead and uncertainty about how the program will execute.
As a result, developers are often told to avoid the STL entirely in those environments.

The inability to use many of C++'s STL features is one of the contributing factors to C's continued dominance in the embedded and kernel space despite C++'s interoperability and near-source compatibility.
This history indicates that an unwinding-based OOM system is likely insufficient for many real users.
If Rust is to be considered a competitor to C in the embedded space, ensuring `alloc` is useful to developers is an important step.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Which features should be automatically set by `cargo` for `core`/`alloc`/`std` dependencies?
  How should which set they are in (user set or cargo set) be exposed to users?
  This is mostly a question for `-Z build-std`, but the desire to disable default features in this case emphasizes it.
- What should the default polarity of Cargo's behavior with regards to accepting an over-featured prebuilt `std` or `alloc` be?

# Future possibilities
[future-possibilities]: #future-possibilities

## Default required features
While not required for this proposal, implementing it and similar proposals would no doubt be made cleaner by the ability to set a feature as required for *most* of a crate, with an item's support in absence of a feature being indicated as a special case.

## Incremental function availability
This proposal suggests initially masking off nearly all of `std` behind this feature and incrementally bringing functions to availability without the `infallible_allocation` feature on an as-needed basis under the supervision of the libs team.
As a result, if this proposal were accepted it would likely be followed by a number of smaller discussions about creating fallible APIs for functionality developers want.

## Default dependencies for workspaces
If this proposal goes through, workspaces for the use-cases listed in the motivation section will likely need to explicitly annotate *every* crate in their environment as using restricted features.
We could add a way to select at a workspace level an environment with a restricted set of feature flags enabled for sysroot crates.
