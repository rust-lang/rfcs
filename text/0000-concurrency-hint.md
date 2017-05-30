- Feature Name: `concurrency_hint`
- Start Date: 2015-03-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `concurrency_hint` function to the `std::env` module to indicate a
**hint** as to the amount of concurrency the underlying physical hardware may
support.

# Motivation

Right now the old `std::os::num_cpus` function is deprecated and on its way out.
As [previous RFCs][stdenv] have noted, the notion of a CPU is somewhat difficult
to define. Committing to the name `num_cpus` would perhaps be an adverse
committment we don't necessarily want to retain into the future.

[stdenv]: https://github.com/rust-lang/rfcs/pull/578#discussion_r22839432

As [others][c1] [have][c2] [commented][c3], though, the API is still in use
today and may want to be stabilized. As a result, this RFC is an attempt to
stabilize this functionality in a form that we are comfortable committing to.

[c1]: http://www.reddit.com/r/rust/comments/2z9iqp/weve_got_basically_all_of_the_large_api_areas/cph3ezl
[c2]: http://users.rust-lang.org/t/using-unstable-apis-tell-us-about-it/157/54?u=alexcrichton
[c3]: http://users.rust-lang.org/t/using-unstable-apis-tell-us-about-it/157/57?u=alexcrichton

# Detailed design

This function and documentation will be added to the `std::env` module of the
standard library. The implementation will be the exact same that `num_cpus`
currently has today.

```rust
/// Returns a *hint* to the number of concurrent threads supported.
///
/// The returned number is not guaranteed to be the number of physical or
/// logical cores but instead simply a guide of how many concurrent threads of
/// execution may run.
///
/// It is guaranteed that the returned value will never be 0.
pub fn concurrency_hint() -> u32;
```

This function is inspired by [@huonw's comment][huon] which is in turn inspired
by [C++'s `thread::hardware_concurrency`][cpp] function. The key difference
between this name and the previous is that the term "hint" is strongly conveyed
and the notion of the number of cpus is detached.

[huon]: http://www.reddit.com/r/rust/comments/2z9iqp/weve_got_basically_all_of_the_large_api_areas/cphkcyh
[cpp]: http://en.cppreference.com/w/cpp/thread/thread/hardware_concurrency

This form of "concurrency hint" versus determining the actual number of cores
seems to be one of the primary use cases for this function, motivating the lack
of functionality to learn a precise statistic about the current hardware (e.g.
number of physical and logical cores).

# Drawbacks

* This functionality is [already available][crates] on crates.io.
* Exposing a "concurrency hint" may not be as widely useful as knowing a
  concrete statistic about the underlying system. For example this function is
  largely only relevant for determining the amount of parallelism to use, not
  for display back to a user.

[crates]: https://crates.io/crates/num_cpus

# Alternatives

The primary alternative is to explore the design space of what it means to learn
about the number of cores on a system. This would then likely guide us towards a
number of APIs to learn about the underlying system and the number of cores it
has (or just general concurrency support). This alternative has not been greatly
pursued, however, and it is unclear how it would play out.

# Unresolved questions

* Is the exact implementation of `num_cpus` as it is today appropriate for this
  new function?
