- Feature Name: 
- Start Date: Mon Mar 23 17:56:42 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Partially revert "Stability as a Deliverable" by allowing users of the stable
compiler to opt into unstable features and standard library functions.

# History

>Rust is a systems programming language [...]

from http://www.rust-lang.org

---

On 2014-09-15 the post [Road to Rust
1.0](http://blog.rust-lang.org/2014/09/15/Rust-1.0.html)
was published on the rust-lang blog. The following quotes are from this post.

>After 1.0 is released, future 1.x releases will be backwards compatible,
>meaning that existing code will continue to compile unmodified [...]

<!-- -->
>Basically, it means that we think the design of Rust finally feels right. [...]
>This is very exciting, because any library we can write, you can write too.

<!-- -->
>[...] it is even possible to write an operating systems kernel in Rust

<!-- -->
>Rust has remained true to its goal of [...] offering the efficiency and
>low-level control that C and C++ offer. Basically, if you want to get your
>hands dirty with the bare metal machine, but you don't want to spend hours
>tracking down segfaults and data races, Rust is the language for you.

On 2014-10-30, hovever, the post [Stability as a
Deliverable](http://blog.rust-lang.org/2014/10/30/Stability.html) made it clear
that none of this will be true in the 1.0 release.

>If your code compiles on Rust stable 1.0, it should compile with Rust stable
>1.x **with a minimum of hassle**.

<!-- -->
>New features and new APIs will be flagged as unstable via feature gates and
>stability attributes, respectively. Unstable features and standard library APIs
>will only be available on the nightly branch, and only if you explicitly "opt
>in" to the instability.

Regarding opting into instability it says

>First, as the web has shown numerous times, merely advertising instability
>doesn't work. Once features are in wide use it is very hard to change them

<!-- -->
>Second, unstable features are by definition work in progress.

<!-- -->
>Finally, we simply cannot deliver stability for Rust unless we enforce it.
>[...] If libraries could opt in to instability, then we could only keep this
>promise if all library authors guaranteed the same thing by supporting all
>three release channels simultaneously.

<!-- -->
>It's not realistic or necessary for the entire ecosystem to flawlessly deal
>with these problems. Instead, we will enforce that stable means stable: the
>stable channel provides only stable features.

# Systems programming languages

>A system programming language usually refers to a programming language used for
>system programming; such languages are designed for writing system software,
>which usually requires different development approaches when compared to
>application software.

>System software is computer software designed to operate and control the
>computer hardware, and to provide a platform for running application software.
>System software includes software categories such as operating systems, utility
>software, device drivers, compilers, and linkers.

-- from Wikipedia

One feature one might expect from systems languages is the following:

**It's possible to use the system without another compiler.**

This will not be possible in Rust 1.0. For example: On Linux, Rust programmers
have to link against the glibc to do anything. At least two features would be
necessary to avoid this:

- Inline assembly
- Not using the standard library

Rust cannot claim that it is a systems language if it depends on a C library for
the systems part.

# Compiler stability

The post "Stability as a Deliverable" made some claims which we will now refute.

>First, as the web has shown numerous times, merely advertising instability
>doesn't work. Once features are in wide use it is very hard to change them

A compiler is not "the web." A compiler change does not directly affect
end-users. When a compiler makes a backwards-incompatible change, the people
that are directly affected are those that opted into using the, now broken,
code. The post already made it clear that stable Rust will not follow semver and
that code will break between minor updates.

>Second, unstable features are by definition work in progress.

It can easily be seen that this is an incorrect generalization. For example, the
`breakpoint` intrinsic which inserts a debugger breakpoint can hardly do
anything else. Somewhat more complicated: The details of inline assembly might
change, but this can only affect the frame of the inline assembly. The assembly
itself will stay the same. The `thread_local` attribute which is also unstable
can only create thread local storage. The atomic intrinsics can only do atomic
operations.

Those are well known features of systems languages that are not at all work in
progress but map directly to LLVM functions which are already used in other
languages.

>Finally, we simply cannot deliver stability for Rust unless we enforce it.
>[...] If libraries could opt in to instability, then we could only keep this
>promise if all library authors guaranteed the same thing by supporting all
>three release channels simultaneously.

This idea of stability can only exist if there is only one Rust compiler. Once
there is a second compiler that can consume rustc output, trying to keep the
thumb down on language features will mean that people move to other compilers
that offer what they need. If Mozilla wants this kind of stability, then it
follows that Mozilla also wants a monopoly on the whole language and compilers.
This is in contrast to other systems languages such as C and C++ that offer many
compilers for many platforms which encourages competition.

The negative consequences of the quasi monopoly that currently exists are
readily apparent: Thread local variables, which have been a standard feature in
systems languages for many years, are feature gated and people are actively
discouraged from using them because they might not be available on all
platforms. Mozilla takes the freedom to use unstable features in their
libraries, knowing that you can't write many things without them, but doesn't
allow other people to write competing libraries.  Some even suggest that Rust
will never have atomic operations on 32 bit variables because this might not be
portable to all systems they want to use Rust on.

Mozilla, being mainly interested in using Rust for their cross platform browser,
is, of course, less interested in platform specific features or systems
programming. It's only natural that they don't want any of the packages they use
in their browser to ever break. Given that many (or even most) of the people in
charge of steering the project are employed by Mozilla, this causes features
which are necessary for systems development to be disregarded.

The language used to justify this idea of stability hints at this: "On the Web
it like this", "We use the browser release model".

>It's not realistic or necessary for the entire ecosystem to flawlessly deal
>with these problems. Instead, we will enforce that stable means stable: the
>stable channel provides only stable features.

It is, however, necessary to allow the people who were drawn in by the claim
that Rust is a systems language to use it as a systems language.

# Compiler versions

The first argument against this RFC will be "people who want to use Rust for
systems programming can use the nightlies." This is possible for local
development but has many downsides that make it effectively impossible for
libraries (which are one of the applications of systems languages).

First: Most platforms will only have stable compilers available. Many platforms
will only have old compiler versions available, making 1.0 the standard people
have to develop against.

Second: People that don't develop against the stable compiler effectively
opt out of cargo.

# Detailed design

Allow people to opt into unstable language features in the stable and beta
releases. Allow people to opt out of unstable language features by having the
compilation abort if one of the dependencies uses unstable features.

Decide whether unstable crates should be allowed on crates.io.

# Drawbacks

Some libraries will not be usable in "stable-only" programs. This puts pressure
on the language developers to stabilize the features people actually use.

# Alternatives

Don't do this and remove the claim that Rust 1.0 is a systems language from the
rust-lang website.
