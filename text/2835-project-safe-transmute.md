- Feature Name: `project-safe-transmute`
- Start Date: 2019-12-06
- RFC PR: [rust-lang/rfcs#2835](https://github.com/rust-lang/rfcs/pull/2835)
- Rust Issue: N/A

# Summary
[summary]: #summary

To form a project group with the purpose of designing subsequent RFCs around the
topic of safe transmute between types.
* This RFC explicitly builds off of processes introduced in the [FFI unwinding project
group RFC](https://github.com/rust-lang/rfcs/pull/2797/files)
* The primary goal of the group is to determine how to replace most uses of
[`std::mem::transmute`][transmute] with safe alternatives.
  * Subsequent goals may include extending other language features that are made possible
  with safe transmute including safe reading of union fields

# Motivation
[motivation]: #motivation

Transmuting one type to another type and vice versa in Rust is extremely dangerous ---
so much so that the docs for [std::mem::transmute][transmute] are essentially a long
list of how to avoid doing so. However, transmuting is often times necessary especially
in lower level contexts where copy of bytes is prohibitively expensive. For instance,
in extremely performance-sensitive use cases, it may be necessary to transmute from
bytes instead of explicitly copying bytes from a buffer into a struct.

Because of this fact, [many][zerocopy] [external][safe-transmute] [crates][bytemuck]
have been developed to tackle this issue, but no single crate has managed to solidify
itself as a clear favorite in this space. Additionally, while it is possible to improve
on unsafe transmute considerably in libraries, having such facilities in the standard
library opens up the possibility of bringing safe constructs to even more currently
unsafe features.

For these reasons, we plan on learning from the prior art to implement a standard way of
transmuting types in a safe way.

## Details of the safe transmute project group

[Repository][repository]

Initial shepherds:

* [rylev (Ryan)](https://github.com/rylev)

Lang team liaisons:

* [joshtriplett (Josh)](https://github.com/joshtriplett)

### Charter
[charter]: #charter

The safe transmute project group has the following initial scope:

* to define APIs for allowing zero copy transmute between types in a completely
  memory safe manner

Once this scope has been reached, the team may continue working on features that are
natural extensions of safe transmute like safe reading on union fields.

### Constraints and considerations

In its work, the project-group should consider various constraints and
considerations:

* That this feature is meant for performance sensitive workloads
* That safety is of the upmost importance as there is already a way to
  transmute using unsafe APIs

### Participation in the project group

Like any Rust group, the safe transmute project group intends to operate
in a public and open fashion and welcomes participation. Visit the
[repository][repository] for more details.

# Drawbacks
[drawbacks]: #drawbacks

* It is possible that the scope of this endeavor is not large enough to warrant a
  separate project group.
* It can be argued that the design space has not been fully explored as evidenced by
  the many crates that address the issue without one being the clear "go to", and thus
  this issue should be left to libraries for further iteration. We believe that while
  there is no clear winner among existing crates, they are stable enough, small enough
  and share enough implementation characteristics to be ready for the community to
  rally around one design direction in the standard library.

# Prior art
[prior-art]: #prior-art

The formation of the project group was first discussed in the [FFI unwind
project group RFC][ffi unwind]. As is state in that RFC, this working group can be
considered a precursor to the current ["shepherded project group" proposal][shepherd].

# Unresolved questions and Future possibilities
[unresolved-questions]: #unresolved-questions

Since this RFC merely formalizes the creation of the project group, it
intentionally leaves all technical details within the project's scope
unresolved.

# Future possibilities
[future-possibilities]: #future-possibilities

The project group will start with a fairly [limited scope][charter], but if the
initial effort to design and stabilize APIs for safe transmute between types,
there is at least one other area that can be expanded upon by this group: safe reading
of union fields.

[transmute]: https://doc.rust-lang.org/std/mem/fn.transmute.html
[ffi unwind]: https://github.com/rust-lang/rfcs/pull/2797
[zerocopy]: https://docs.rs/zerocopy
[safe-transmute]: https://docs.rs/safe-transmute
[bytemuck]: https://docs.rs/bytemuck
[shepherd]: http://smallcultfollowing.com/babysteps/blog/2019/09/11/aic-shepherds-3-0/
[repository]: https://github.com/rust-lang/project-safe-transmute
