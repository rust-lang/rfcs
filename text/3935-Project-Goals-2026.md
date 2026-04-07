- Feature Name: `project_goals_2026`
- Start Date: 2026-02-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/issues/0000)(https://github.com/rust-lang/rfcs/pull/0000)

# Summary

Establish the initial round of Rust Project Goals for 2026 along with a set of current roadmaps, which describe multi-year development arcs.

New Rust Project Goals may be added over the course of the year but only if all required resources (champions, funding, etc) are already known.

# Motivation

The 2026 goal slate consists of 66 project goals. In comparison to prior rounds, we have changed to *annual* goals rather than six-month goal periods. Annuals goals give us more time to discuss and organize.

## Why we set goals

Goals serve multiple purposes.

For would-be contributors, goals are Rust's front door. If you want to improve Rust - whether that's a new language feature, better tooling, or fixing a long-standing pain point - the goal process is how you turn that idea into reality. When you propose a goal and teams accept it, you get more than approval. You get a *champion* from the relevant team who will mentor you, help you navigate the project, and ensure your work gets the review attention it needs. Goals are a contract: you commit to doing the work, the project commits to supporting you.

For users, goals serve as a roadmap, giving you an early picture of what work we expect to get done this year.

For Rust maintainers, goals help to surface interactions across teams. They aid in coordination because people know what work others are aiming to do and where they may need to offer support.

## Goals are *proposed* by contributors and *accepted* by teams

As an open-source project, Rust's goal process works differently than a company's. In a company, leadership sets goals and assigns employees to execute them. Rust doesn't have employees - we have contributors who volunteer their time and energy. So in our process, goals begin with the *contributor*: the person (or company) that wants to do the work.

Contributors *propose* goals; Rust teams *accept* them. When you propose a goal, you're saying you're prepared to invest the time to make it happen. When a team accepts, they're committing to support that work - doing reviews, engaging in RFC discussions, and providing the guidance needed to land it in Rust.

## How these goals were selected

Goal proposals were collected during the month of January. Many of the goals are continuing goals that are carried over from the previous year, but others goal are new.

In February, an *alpha* version of this RFC is reviewed with teams. Teams vet the goals to determine if they are realistic and to make sure that goal have champions from the team. A *champion* is a Rust team member that will mentor and guide the contributor as they do their work. Champions keep up with progress on the goal, help the champion figure out technical challenges, and also help the contributor to navigate the Rust team(s). Champions also field questions from others in the project who want to understand the goal.

## How to follow along with a goal's progress

Once the Goals RFC is accepted, you can follow along with the progress on a goal in a few different ways:

* Each goal has a tracking issue. Goal contributors and champions are expected to post regular updates. These updates are also posted to Zulip in the `#project-goals` channel.
* Regular blog posts cover major happenings in goals.

# Guide-level explanation

There are a total of 66 planned for this year. That's a lot! You can see the complete list, but to help you get a handle on it, we've selected a few to highlight. These are goals that will be stabilizing this year or which we think people will be particularly excited to learn about.

**Important:** You have to understand the nature of a Rust goal. Rust is an open-source project, which means that progress only happens when contributors come and *make* it happen. When the Rust project declares a goal, that means that (a) contributors, who we call the *task owners*, have said they want to do the work and (b) members of the Rust team members have promised to support them. Sometimes those task owners are volunteers, sometimes they are paid by a company, and sometimes they supported by grants. But no matter which category they are, if they ultimately are not able to do the work (say, because something else comes up that is higher priority for them in their lives), then the goal won't happen. That's ok, there's always next year!

## Running Rust scripts will get more convenient with *cargo script*

| Goal | What and why |
| --- | --- |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html) | Stabilize support for "cargo script", the ability to have a single file that contains both Rust code and a `Cargo.toml`. |

*People involved: [Ed Page][]*

---

"Cargo script" let's you create a single file that specifies both a Rust program and the dependencies it needs and then execute that program with one convenient command. For example, you can now take a Rust file like this:

```rust
#!/usr/bin/env cargo
---
edition: 2024
[dependencies]
reqwest = { version = "0.12", features = ["blocking"] }
---

fn main() {
    let body = reqwest::blocking::get("https://httpbin.org/ip")
        .unwrap()
        .text()
        .unwrap();
    println!("My IP info: {body}");
}
```

and run it with `cargo my_ip.rs`.  Or, thanks to the `#!` line, you can just run `./my_ip.rs`.

This feature makes good use of [one of the things we found when doing our research for the Vision Doc](https://blog.rust-lang.org/2025/12/19/what-do-people-love-about-rust/#but-what-they-love-is-the-sense-of-empowerment-and-versatility): that people love Rust not only because it helps them build foundational software, but because it's a expressive and productive enough that "you can write everything from the top to the bottom of your stack in it" (-- Rust expert and consultant focused on embedded and real-time systems). Until now, the fly in the ointment was that packaging up a Rust package required several files and required people to do a separate compilation step. Cargo script solves that problem.

## The borrow checker will be more flexible with *Polonius alpha*

| Goal | What and why |
| --- | --- |
| [Stabilize polonius alpha](https://rust-lang.github.io/rust-project-goals/2026/polonius.html#stabilize-polonius-alpha) | Fix remaining issues, validate on real-world code, and ship a stable improved borrow checker. |
| [Extend formality for Polonius alpha](https://rust-lang.github.io/rust-project-goals/2026/polonius.html#extend-formality-for-polonius-alpha) | Build a formal model of borrow checking in a-mir-formality and upstream it into the Rust reference. |

*People involved: [Jack Huey][], [Rémy Rakic][], [Niko Matsakis][], [tiif][], [Amanda Stjerna][]*

---

The "Polonius Alpha" work represents the final completion of the original promise from the [2018 Non-lexical Lifetimes RFC](https://rust-lang.github.io/rfcs/2094-nll.html). That RFC originally planned to address three problematic patterns -- but ultimately, for efficiency reasons, we were only able to fix two. In the meantime, for the last several years, we have been pursuing work on Polonius, a next generation borrow checker formulation, that aims to close this gap and more.

The Polonius Alpha rules extend the borrow checker to accept the so-called ["Problem Case #3"](https://rust-lang.github.io/rfcs/2094-nll.html#problem-case-3-conditional-control-flow-across-functions) that NLL ultimately failed to solve. This case occurs when you have conditional control flow across functions. For example, in this case the call to `map.get_mut(&key)`, the borrow of `map` is only "live" in the `Some` branch, where it is returned (and hence must outlive `'r`). But because of imprecision in the borrow checker, the borrow winds up being enforced in the `None` branch as well, resulting in an error:

```rust
fn get_default<'r,K:Hash+Eq+Copy,V:Default>(
    map: &'r mut HashMap<K,V>,
    key: K,
) -> &'r mut V {
    match map.get_mut(&key) { // ──────────────────┐ 'r only needs to
        Some(value) => value,              // ◄────┘ be valid here...
        None => {                          //      │
            map.insert(key, V::default()); //      │
            //  ^~~~~~ ERROR               //      │
            map.get_mut(&key).unwrap()     //      │
        }                                  //      │
    }                                      //      │ ...but today it covers
}                                          // ◄────┘ all this
```

Under Polonius Alpha, this code compiles.

Polonius Alpha is part of a larger roadmap called [the Borrow-Checker Within](https://rust-lang.github.io/rust-project-goals/2026/roadmap-borrow-checker-within.html) that we expect to be driving over the next few years. This year, another part of that work is including Polonius Alpha in [a-mir-formality](https://github.com/rust-lang/a-mir-formality/), the [types team's](https://rust-lang.org/governance/teams/compiler/#team-types) (in-progress) specification for how the Rust type system works. As part of another goal, we are planning to [integrate a-mir-formality into the Rust reference](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html). This would make Polonius the first version of the borrow checker whose behavior is specified outside of the Rust compiler.

## Change const evaluation to support *traits*, and *reflection*, allow *structs/enums* as const parameter types

| Goal | What and why |
| --- | --- |
| [ADT const params](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html#adt-const-params) | Support structs, tuples, arrays in const generics. |
| [Min generic const arguments](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html#min-generic-const-arguments) | Support associated constants and generic parameters embedded in other expressions. |
| [Stabilize const traits MVP](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html#stabilize-const-traits-mvp) | Finalize the RFC, complete the compiler implementation, and stabilize so `const fn` can call trait methods. |
| [Explore design space for comptime `const fn`](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html#explore-design-space-for-comptime-const-fn) | Implement and validate `#[compile_time_only]` attribute for `const fn` that enables type reflection without runtime overhead. |

*People involved: [Boxy][], [Deadbeef][], [Josh Triplett][], [Niko Matsakis][], [Oliver Scherer][], [Scott McMurray][], [TC][]*

---

This year we'll be extending Rust's support for const evaluation in several ways. To start, you'll be able to use structs and enums as the values for const generics, not only integers. So where today you can write `Array<3>`, you'll be able to write something like this:

```rust
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

pub fn process<const D: Dimensions>(data: &[f32]) {
    // ...
}

fn main() {
    process::<{ Dimensions { width: 1920, height: 1080 } }>(&data);
}
```

You'll also be able to use associated constants as const generic arguments, like `Buffer<T::MAX_SIZE>`.

Next, we are integrating `const` into the trait system. When you implement a trait, you'll be able to provide a `const` impl which means that the methods in the trait are all const-compatible. `const fn` can then use bounds like `T: const Display` to indicate that they need a type with a const-compatible impl or `T: [const] Display` to indicate that they need a const-compatible impl when called in a const context. Const traits are particularly helpful because they allow you to use builtin language constructs like `?` and `for` loops:

```rust
const fn sum_up<I: [const] Iterator<Item = i32>>(iter: I) -> i32 {
    let mut total = 0;
    for val in iter {
        total += val;
    }
    total
}
```

Finally, we're beginning early experimental work on compile-time reflection — the ability for const functions to inspect type information. It's too early to promise specifics, but the long-term vision is things like serialization working without derive macros.

## Ergonomic ref-counting and (maybe) async traits

| Goal | What and why |
| --- | --- |
| [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html) | Enable dyn dispatch for async traits via `.box` notation |
| [Add a `Share` trait](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html#add-a-share-trait) | A trait that identifies types where cloning creates an alias to the same underlying value, like `Arc`, `Rc`, and shared references. |
| [Support `move(...)` expressions in closures](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html#support-move-expressions-in-closures) | Precise control over what closures capture and when, eliminating the need for awkward clone-into-temporary patterns. |

*People involved: [Takayuki Maeda][], [Niko Matsakis][], [Santiago Pastorino][]*

---

We have a lot of ongoing plans to improve the async Rust experience, but the two most likely to hit stable are [more ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html) and [extensions to async fn in traits](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html).

The [ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html) discussion has gone through [many stages](https://smallcultfollowing.com/babysteps/series/ergonomic-rc/), but one solid step everyone agrees on is making it (a) more obvious when you are sharing two handles to the same object vs doing a deep clone, via the `Share` trait, and (b) more ergonomic to capture clones into closures and async blocks with `move($expr)` expressions:

```rust
// Today: awkward temporary variables
let tx_clone = tx.clone(); // am I deep cloning or what?
tokio::spawn(async move {
    send_data(tx_clone).await;
});

// With Share + move expressions: inline and clear
tokio::spawn(async {
    send_data(move(tx.share())).await;
}); //        ---------------- capture a shared handle
```

We also plan to cut a "practical path" to support [invoking async fns through `dyn Trait`](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html). The initial version would be limited to boxed futures but the goal is to be forwards-compatible with the ongoing [in-place initialization](https://rust-lang.github.io/rust-project-goals/2026/in-place-init.html) designs for non-boxed allocation (e.g., stack). The RFC for this hasn't been written yet, and the proposal includes some new syntax, so that could be spicy! Stay tuned.

## Try, never, extern types, oh my!

| Goal | What and why |
| --- | --- |
| [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html) | Over the next year, we will build on the foundational work from 2025 to stabilize the `Sized` trait hierarchy and continue nightly support for scalable vectors: |
| [Stabilize never type (`!`)](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html) | Stabilize the never type aka `!`. |
| [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html) | Stabilize the `Try` trait, which customizes the behavior of the `?` operator. |

*People involved: [Amanieu d'Antras][], [waffle][], [David Wood][], [Jana Dönszelmann][], [lcnr][], [Niko Matsakis][], [Tyler Mandry][]*

---

Three long-awaited features are making their way toward stabilization this year.

The [`Try` trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html) customizes the behavior of the `?` operator, letting you use it with your own types beyond `Result` and `Option`. For example, you could define a `TracedResult` that automatically captures the source location each time an error is bubbled up with `?`:

```rust
fn read_list(path: PathBuf) -> TracedResult<Vec<i32>> {
    let file = File::open(path)?;  // captures location
    Ok(read_number_list(file)?)    // captures location
}
```

No more choosing between readable error handling and useful diagnostics.

The [never type `!`](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html) has been unstable for *ten years*. It represents computations that never produce a value — like functions that always panic or loop forever. The final blockers are being resolved, and stabilization is in sight.

Finally, the [Sized trait hierarchy](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html) work will stabilize a richer set of sizing traits, which unblocks [extern types](https://github.com/rust-lang/rfcs/pull/1861) — another long-requested feature. Today, `?Sized` conflates "unsized but has metadata" with "truly sizeless." The new hierarchy distinguishes these cases. This same work is also laying the foundation for scalable vector support (Arm SVE), where vector sizes depend on the CPU rather than being fixed at compile time.

## Going "beyond the `&`" with better integration for custom pointer types

| Goal | What and why |
| --- | --- |
| [Arbitrary self types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html#arbitrary-self-types) | Let user-defined smart pointers work as method receivers, stabilizing the `arbitrary_self_types` feature. |
| [`Deref`/`Receiver` split chain experiment](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html#deref-receiver-split-chain-experiment) | Experiment with letting `Receiver::Target` and `Deref::Target` diverge, collecting data on utility and use cases. |
| [`derive(CoercePointee)`](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html#derive-coercepointee) | Support `dyn Trait` coercion for user-defined smart pointers. |
| [Map the field projection design space](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html#map-the-field-projection-design-space) | Explore the virtual places approach, document it in the beyond-refs wiki, formalize borrow checker integration, and build a compiler experiment. |

*People involved: [Benno Lossin][], [Alice Ryhl][], [Ding Xiang Fei][], [Jack Huey][], [Rémy Rakic][], [Niko Matsakis][], [Oliver Scherer][], [Tyler Mandry][], [TC][]*

---

Two goals this year are working to make it possible for user-defined types to be used in all the ways that you can use `Box`, `Arc`, and `&`.

[Arbitrary self types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html) lets you use custom smart pointers as method receivers. With the `Receiver` trait and `derive(CoercePointee)`, your pointer types work just like `Box` or `Arc` — including method dispatch and coercion to `dyn Trait`:

```rust
impl Person {
    fn biometrics(self: &SmartPointer<Self>) -> &Biometrics {
        ...
    }
}

let person: SmartPointer<Person> = get_data();
let bio = person.biometrics(); // just works
```

We are also continuing our experimental work to support [custom field projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html) — accessing fields *through* a smart pointer. Today, `&x.field` gives you `&Field`, but there's no equivalent for `NonNull`, `Pin`, or custom pointer types. The field projections design is exploring a "virtual places" approach that would make this work generically. The goal for this year is a compiler experiment on nightly and draft RFCs, with the [beyond-refs wiki](https://rust-lang.github.io/beyond-refs/) documenting the design space.

Both of these goals spun out from the ongoing work to support the needs of the [Rust for Linux](https://rust-lang.github.io/rust-project-goals/2026/roadmap-rust-for-linux.html) project and are part of the [Beyond the `&`](https://rust-lang.github.io/rust-project-goals/2026/roadmap-beyond-the-ampersand.html) roadmap.

## Build it your way with build-std

| Goal | What and why |
| --- | --- |
| [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html) | Let Cargo rebuild the standard library from source for custom targets and configurations |

*People involved: [David Wood][], [Eric Huss][]*

---

A new version of [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html) is expected to hit nightly this year. Build-std lets Cargo rebuild the standard library from source, which unlocks things like using std with tier three targets, rebuilding with different codegen flags, and stabilizing ABI-modifying compiler flags. It's particularly valuable for embedded developers, where optimizing for size matters and targets often don't ship with a pre-compiled std.

An unstable `-Zbuild-std` flag has existed for a while, but this new design — progressing through a series of RFCs ([one accepted](https://github.com/rust-lang/rfcs/pull/3873), [two more](https://github.com/rust-lang/rfcs/pull/3874) [in review](https://github.com/rust-lang/rfcs/pull/3875)) — has a path to stabilization. Build-std is also part of the [Rust for Linux](https://rust-lang.github.io/rust-project-goals/2026/roadmap-rust-for-linux.html) roadmap.

## Closing soundness bugs and supporting new lang features with a new trait solver

| Goal | What and why |
| --- | --- |
| [Stabilize the next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html) | Replace the existing trait solver with a sound, maintainable implementation that unblocks soundness fixes and async features |

*People involved: [lcnr][], [Niko Matsakis][]*

---

This year, the Rust types team plans to stabilize the [next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html). This solver is a ground-up rewrite of the core engine that decides whether types satisfy trait bounds, normalizes associated types, and more. The types team has been working on it since late 2022, and it already powers coherence checking as of Rust 1.84. The goal for this year is to stabilize it for use across all of Rust and remove the old implementation.

This goal may not *sound* like it's going to impact your life, but finishing the new solver unblocks a *lot* of stuff. To start, it allows us to make progress on the [Project Zero](https://rust-lang.github.io/rust-project-goals/2026/roadmap-project-zero.html) roadmap, which aims to fix every known type system soundness bug. It also unblocks long-desired features like implied bounds, cyclic trait matching, and features needed by the [Just add async](https://rust-lang.github.io/rust-project-goals/2026/roadmap-just-add-async.html) roadmap.
Roadmaps offer a "zoomed out" view of the Rust project direction. Each roadmap collects a set of related project goals into a coherent theme. A typical roadmap takes several years to drive to completion, so when you look at the roadmap, you'll see not only the work we expect to do this year, but a preview of the work we expect in future years (to the extent we know that).

## Active roadmaps

Not every goal is part of a roadmap, nor are they all expected to be. This initial set of roadmaps is based on the trends that we saw in the 2026 goals. Over time, we expect to add more roadmaps and refine their structure to help people quickly see where Rust is going.

| Roadmap                                                       | Point of contact | What and why                                                                                                                                |
| :--                                                           | :--           | :--                                                                                                                                         |
| [Beyond the `&`](https://rust-lang.github.io/rust-project-goals/2026/roadmap-beyond-the-ampersand.html)             | [Tyler Mandry][]      | Smart pointers (`Arc`, `Pin`, FFI wrappers) get the same ergonomics as `&` and `&mut` — reborrowing, field access, in-place init            |
| [The Borrow Checker Within](https://rust-lang.github.io/rust-project-goals/2026/roadmap-borrow-checker-within.html) | [Niko Matsakis][] | Make the borrow checker's rules visible in the type system — place-based lifetimes, view types, and internal references built on Polonius   |
| [Constify all the things](https://rust-lang.github.io/rust-project-goals/2026/roadmap-constify-all-the-things.html) | [Oliver Scherer][]      | Const generics accept structs and enums; compile-time reflection means `serialize(&my_struct)` works without derives                        |
| [Just add async](https://rust-lang.github.io/rust-project-goals/2026/roadmap-just-add-async.html)                   | [Niko Matsakis][] | Patterns that work in sync Rust should work in async Rust — traits, closures, drop, scoped tasks                                            |
| [Project Zero](https://rust-lang.github.io/rust-project-goals/2026/roadmap-project-zero.html)                       | [lcnr][]         | Fix all known type system unsoundnesses so Rust's safety guarantees are actually reliable                                                   |
| [Rust for Linux](https://rust-lang.github.io/rust-project-goals/2026/roadmap-rust-for-linux.html)                   | [Tomas Sedovic][] | Build the Linux kernel with only stable language features.                                                                                  |
| [Safety-Critical Rust](https://rust-lang.github.io/rust-project-goals/2026/roadmap-safety-critical-rust.html)       | [Pete LeVasseur][]   | MC/DC coverage, a specification that tracks stable releases, and `unsafe` documentation — the evidence safety assessors need                |

# Reference-level explanation

This section contains the complete list of all 66 [goals](#goals) for 2026. There are a lot of them! You may prefer to look at the [roadmaps](https://rust-lang.github.io/rust-project-goals/2026/roadmaps.html) to get a higher level picture of where Rust is going.



## Goals

### Goals by size

#### Large goals

Large goals require the engagement of entire team(s). The teams that need to engage with the goal are highlighted in bold.

| Goal                                                                                       | PoC               | Team                      | Champion          |
| :--                                                                                        | :--               | :--                       | :--               |
| [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                            | [Ding Xiang Fei][] | **[types][]**             | [Jack Huey][]         |
|                                                                                            |                   | [lang-docs][]             | [TC][]      |
|                                                                                            |                   | [lang][]                  | [Tyler Mandry][]          |
|                                                                                            |                   | [libs-api][]              | *n/a*             |
|                                                                                            |                   | [libs][]                  | *n/a*             |
| [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html)                                                                  | [David Wood][]        | **[cargo][]**             | [Eric Huss][]            |
|                                                                                            |                   | [compiler][]              | *n/a*             |
|                                                                                            |                   | [crates-io][]             | *n/a*             |
|                                                                                            |                   | [libs][]                  | *n/a*             |
| [Full Const Generics](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html)                                                   | [Boxy][]          | **[lang][]**              | [Niko Matsakis][]     |
|                                                                                            |                   | **[types][]**             | [Boxy][]          |
| [Const Traits](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html)                                                            | [Deadbeef][]        | **[lang][]**              | [TC][]      |
|                                                                                            |                   | **[types][]**             | [Oliver Scherer][]          |
|                                                                                            |                   | [compiler][]              | *n/a*             |
| [Architectural groundwork for expansion-time evaluation](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html)     | [Tyler Mandry][]          | **[compiler][]**          | [Vadim Petrochenkov][]     |
|                                                                                            |                   | [types][]                 | [Oliver Scherer][]          |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                  | [Benno Lossin][]      | **[lang][]**              | [Tyler Mandry][]          |
|                                                                                            |                   | [compiler][]              | [Ding Xiang Fei][] |
|                                                                                            |                   | [types][]                 | [Rémy Rakic][]              |
|                                                                                            |                   | [libs-api][]              | *n/a*             |
|                                                                                            |                   | [libs][]                  | *n/a*             |
|                                                                                            |                   | [opsem][]                 | [Mario Carneiro][]             |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)              | [Amanieu d'Antras][]          | **[edition][]**           | [Eric Huss][]            |
|                                                                                            |                   | **[libs-api][]**          | [Amanieu d'Antras][]          |
|                                                                                            |                   | [compiler][]              | [Jane Lusby][]            |
|                                                                                            |                   | [rustdoc][]               | [Guillaume Gomez][]   |
|                                                                                            |                   | [lang][]                  | *n/a*             |
|                                                                                            |                   | [types][]                 | *n/a*             |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html)                                            | [Amanieu d'Antras][]          | **[opsem][]**             | [Ralf Jung][]         |
|                                                                                            |                   | [compiler][]              | ![TBD][]          |
|                                                                                            |                   | [wg-mir-opt][]            | ![TBD][]          |
|                                                                                            |                   | [lang][]                  | *n/a*             |
| [Immobile types and guaranteed destructors](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html)                                 | [Jack Huey][]         | **[lang][]**              | [Jack Huey][]         |
|                                                                                            |                   | **[types][]**             | [lcnr][]             |
| [Stabilize the next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html)                               | [lcnr][]             | **[types][]**             | [lcnr][]             |
|                                                                                            |                   | [lang][]                  | [Niko Matsakis][]     |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2026/parallel-front-end.html)                                      | [Sparrow Li][]       | **[wg-parallel-rustc][]** | [Vadim Petrochenkov][]     |
|                                                                                            |                   | [compiler][]              | *n/a*             |
| [Stabilize and model Polonius Alpha](https://rust-lang.github.io/rust-project-goals/2026/polonius.html)                                          | [Rémy Rakic][]              | **[types][]**             | [Jack Huey][]         |
| [Redesigning `super let`: Flexible Temporary Lifetime Extension](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html) | [dianne][]           | **[lang][]**              | [TC][]      |
|                                                                                            |                   | [compiler][]              | [dianne][]           |
|                                                                                            |                   | [libs][]                  | *n/a*             |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                      | [Oliver Scherer][]          | **[lang][]**              | [Scott McMurray][]         |
|                                                                                            |                   | [compiler][]              | [Oliver Scherer][]          |
|                                                                                            |                   | [libs-api][]              | [Josh Triplett][]     |
|                                                                                            |                   | [types][]                 | *n/a*             |
| [Normative Documentation for Sound `unsafe` Rust](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html)      | [Pete LeVasseur][]       | **[opsem][]**             | [Ralf Jung][]         |
|                                                                                            |                   | [lang-docs][]             | *n/a*             |
|                                                                                            |                   | [lang][]                  | *n/a*             |
|                                                                                            |                   | [libs-api][]              | *n/a*             |
| [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                | [Tyler Mandry][]          | **[types][]**             | [Jack Huey][]         |
|                                                                                            |                   | [lang][]                  | [Tyler Mandry][]          |
|                                                                                            |                   | [compiler][]              | *n/a*             |
|                                                                                            |                   | [libs][]                  | *n/a*             |
|                                                                                            |                   | [opsem][]                 | *n/a*             |
| [Stabilize FLS Release Cadence](https://rust-lang.github.io/rust-project-goals/2026/stabilize-fls-releases.html)                                 | [Pete LeVasseur][]       | **[fls][]**               | [Pete LeVasseur][]       |
|                                                                                            |                   | [spec][]                  | *n/a*             |


#### Medium goals

Medium goals require support from an individual, the team champion.

| Goal                                                                                                                                            | PoC               | Team                            | Champion         |
| :--                                                                                                                                             | :--               | :--                             | :--              |
| [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html)                                                                                                | ![Help Wanted][]  | [compiler][]                    | [Takayuki Maeda][]         |
|                                                                                                                                                 |                   | [lang][]                        | [Niko Matsakis][]    |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Assumptions on Binders](https://rust-lang.github.io/rust-project-goals/2026/assumptions_on_binders.html)                                                                                             | [Boxy][]          | [types][]                       | [Boxy][]         |
| [Async Future Memory Optimisation](https://rust-lang.github.io/rust-project-goals/2026/async-future-memory-optimisation.html)                                                                         | [Ding Xiang Fei][] | [compiler][]                    | [Tyler Mandry][]         |
| [Async statemachine optimisation](https://rust-lang.github.io/rust-project-goals/2026/async-statemachine-optimisation.html)                                                                           | @diondokter       | [compiler][]                    | [Eric Holk][]           |
| [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                                                                           | [Ian McCormack][]         | [compiler][]                    | [Ralf Jung][]        |
|                                                                                                                                                 |                   | [opsem][]                       | [Ralf Jung][]        |
|                                                                                                                                                 |                   | [infra][]                       | *n/a*            |
|                                                                                                                                                 |                   | [lang][]                        | *n/a*            |
| [Cargo cross workspace cache](https://rust-lang.github.io/rust-project-goals/2026/cargo-cross-workspace-cache.html)                                                                                   | [Ross Sullivan][]      | [cargo][]                       | [Ed Page][]           |
| [Dictionary Passing Style Experiment](https://rust-lang.github.io/rust-project-goals/2026/dictionary-passing-style-experiment.html)                                                                   | [@Nadrieril][]        | [types][]                       | [lcnr][]            |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html)                                                                                                       | ![Help Wanted][]  | [lang][]                        | [Niko Matsakis][]    |
|                                                                                                                                                 |                   | [compiler][]                    | *n/a*            |
|                                                                                                                                                 |                   | [lang-docs][]                   | *n/a*            |
|                                                                                                                                                 |                   | [libs-api][]                    | *n/a*            |
| [Case study for experimental language specification, with integration into project teams and processes](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html) | [Jack Huey][]         | [lang][]                        | [Josh Triplett][]    |
|                                                                                                                                                 |                   | [types][]                       | [Jack Huey][]        |
|                                                                                                                                                 |                   | [spec][]                        | *n/a*            |
| [Project goal - High-Level ML optimizations](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html)                                                                                  | [Manuel Drehwald][]           | [compiler][]                    | [Oliver Scherer][]         |
|                                                                                                                                                 |                   | [lang][]                        | [TC][]     |
|                                                                                                                                                 |                   | [infra][]                       | *n/a*            |
| [Improve `rustc_codegen_cranelift` performance](https://rust-lang.github.io/rust-project-goals/2026/improve-cg_clif-performance.html)                                                                 | [bjorn3][]           | [compiler][]                    | [bjorn3][]          |
|                                                                                                                                                 |                   | [cargo][]                       | *n/a*            |
| [In-place initialization](https://rust-lang.github.io/rust-project-goals/2026/in-place-init.html)                                                                                                     | [Alice Ryhl][]         | [lang][]                        | [Alice Ryhl][]        |
| [Incremental Systems Rethought](https://rust-lang.github.io/rust-project-goals/2026/incremental-system-rethought.html)                                                                                | [Alejandra González][]          | [compiler][]                    | [Jack Huey][]        |
| [Declarative (`macro_rules!`) macro improvements](https://rust-lang.github.io/rust-project-goals/2026/macro-improvements.html)                                                                        | [Josh Triplett][]     | [lang][]                        | [Josh Triplett][]    |
| [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                            | ![Help Wanted][]  | [compiler][]                    | [Oliver Scherer][]         |
|                                                                                                                                                 |                   | [lang][]                        | [@Nadrieril][]       |
|                                                                                                                                                 |                   | [opsem][]                       | [Crystal Durham][]            |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Implement and Maintain MC/DC Coverage Support](https://rust-lang.github.io/rust-project-goals/2026/mcdc-coverage-support.html)                                                                       | @RenjiSann        | [compiler][]                    | [David Wood][]       |
|                                                                                                                                                 |                   | [infra][]                       | *n/a*            |
| [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                                                                        | [walterhpearce][]    | [cargo][]                       | [Arlo Siemsen][]          |
|                                                                                                                                                 |                   | [infra][]                       | [Mark Rousskov][] |
|                                                                                                                                                 |                   | [rustup][]                      | [Dirkjan Ochtman][]             |
|                                                                                                                                                 |                   | [crates-io][]                   | *n/a*            |
| [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                     | @kupiakos         | [compiler][]                    | [Mads Marquart][]         |
|                                                                                                                                                 |                   | [lang][]                        | [Scott McMurray][]        |
|                                                                                                                                                 |                   | [libs][]                        | *n/a*            |
|                                                                                                                                                 |                   | [opsem][]                       | [Connor Horman][]            |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Nightly support for function overloading in FFI bindings](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html)                                                              | @ssbr             | [lang][]                        | [Tyler Mandry][]         |
|                                                                                                                                                 |                   | [compiler][]                    | *n/a*            |
|                                                                                                                                                 |                   | [libs-api][]                    | *n/a*            |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2026/pin-ergonomics.html)                                                                               | [Frank King][]       | [compiler][]                    | [Oliver Scherer][]         |
|                                                                                                                                                 |                   | [lang][]                        | [TC][]     |
|                                                                                                                                                 |                   | [types][]                       | [Oliver Scherer][]         |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2026/pub-priv.html)                                                                                            | ![Help Wanted][]  | [compiler][]                    | [Vadim Petrochenkov][]    |
|                                                                                                                                                 |                   | [cargo][]                       | *n/a*            |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html)                                                                                                           | [Aapo Alasuutari][]         | [lang][]                        | [Tyler Mandry][]         |
|                                                                                                                                                 |                   | [compiler][]                    | *n/a*            |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Prepare TAIT + RTN for stabilization](https://rust-lang.github.io/rust-project-goals/2026/rtn.html)                                                                                                  | ![Help Wanted][]  | [lang][]                        | [TC][]     |
|                                                                                                                                                 |                   | [types][]                       | [lcnr][]            |
| [Stabilize Rust for Linux compiler features](https://rust-lang.github.io/rust-project-goals/2026/rust-for-linux-compiler-features.html)                                                               | [Tomas Sedovic][]     | [compiler][]                    | [Wesley Wiser][]     |
| [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                     | [David Wood][]        | [compiler][]                    | [David Wood][]       |
|                                                                                                                                                 |                   | [lang][]                        | [Niko Matsakis][]    |
|                                                                                                                                                 |                   | [libs-api][]                    | [Amanieu d'Antras][]         |
|                                                                                                                                                 |                   | [types][]                       | [lcnr][]            |
| [Stabilize MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2026/stabilization-of-sanitizer-support.html)                                                  | [Jakob Koschel][]        | [compiler][]                    | [Ramon de C Valle][]         |
|                                                                                                                                                 |                   | [project-exploit-mitigations][] | [Ramon de C Valle][]         |
|                                                                                                                                                 |                   | [bootstrap][]                   | *n/a*            |
|                                                                                                                                                 |                   | [infra][]                       | *n/a*            |
| [Stabilize Cargo SBOM precursor](https://rust-lang.github.io/rust-project-goals/2026/stabilize-cargo-sbom.html)                                                                                       | ![Help Wanted][]  | [cargo][]                       | [Weihang Lo][]       |
| [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                                                                                     | [Tyler Mandry][]          | [lang][]                        | [Tyler Mandry][]         |
|                                                                                                                                                 |                   | [libs-api][]                    | [Amanieu d'Antras][]         |
|                                                                                                                                                 |                   | [compiler][]                    | *n/a*            |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Implement Supertrait `auto impl`](https://rust-lang.github.io/rust-project-goals/2026/supertrait-auto-impl.html)                                                                                     | [Ding Xiang Fei][] | [lang][]                        | [Taylor Cramer][]        |
|                                                                                                                                                 |                   | [types][]                       | *n/a*            |
| [Explicit tail calls & `loop_match`](https://rust-lang.github.io/rust-project-goals/2026/tail-call-loop-match.html)                                                                                   | [Folkert de Vries][]       | [lang][]                        | [Scott McMurray][]        |
|                                                                                                                                                 |                   | [compiler][]                    | *n/a*            |
| [Wasm Components](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html)                                                                                                           | [Yoshua Wuyts][]      | [compiler][]                    | [Wesley Wiser][]     |
|                                                                                                                                                 |                   | [lang][]                        | *n/a*            |
|                                                                                                                                                 |                   | [libs][]                        | *n/a*            |


#### Small goals

Small goals are covered by standard team processes and do not require dedicated support from anyone.

| Goal                                                                                                                                              | PoC              | Team                   | Champion |
| :--                                                                                                                                               | :--              | :--                    | :--   |
| [Expanding a-mir-formality to work better as a Rust type system spec](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html)                                                         | [Jack Huey][]        | [lang-docs][]          | *n/a* |
|                                                                                                                                                   |                  | [spec][]               | *n/a* |
|                                                                                                                                                   |                  | [types][]              | *n/a* |
| [AArch64 Pointer Authentication using aarch64-unknown-linux-pauthtest target on Linux ELF platforms](https://rust-lang.github.io/rust-project-goals/2026/aarch64_pointer_authentication_pauthtest.html) | @jchlanda        | [compiler][]           | *n/a* |
| [Stabilize Cargo's linting system](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html)                                                                                                | [Ed Page][]           | [cargo][]              | *n/a* |
|                                                                                                                                                   |                  | [clippy][]             | *n/a* |
|                                                                                                                                                   |                  | [compiler][]           | *n/a* |
| [Prototype a new set of Cargo "plumbing" commands](https://rust-lang.github.io/rust-project-goals/2026/cargo-plumbing.html)                                                                             | ![Help Wanted][] | [cargo][]              | *n/a* |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html)                                                                                                         | [Ed Page][]           | [cargo][]              | *n/a* |
|                                                                                                                                                   |                  | [compiler][]           | *n/a* |
|                                                                                                                                                   |                  | [lang][]               | *n/a* |
|                                                                                                                                                   |                  | [rustdoc][]            | *n/a* |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html)                                                | [Predrag Gruevski][]      | [cargo][]              | *n/a* |
|                                                                                                                                                   |                  | [rustdoc][]            | *n/a* |
| [Improving Unsafe Code Documentation in the Rust Standard Library](https://rust-lang.github.io/rust-project-goals/2026/improve-std-unsafe.html)                                                         | @hxuhack         | [libs][]               | *n/a* |
|                                                                                                                                                   |                  | [opsem][]              | *n/a* |
| [Interactive cargo-tree: TUI for Cargo's dependency graph visualization](https://rust-lang.github.io/rust-project-goals/2026/interactive-cargo-tree.html)                                               | @orhun           | [cargo][]              | *n/a* |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                                                                                  | [teor][]        | [compiler][]           | *n/a* |
|                                                                                                                                                   |                  | [lang][]               | *n/a* |
|                                                                                                                                                   |                  | [libs-api][]           | *n/a* |
|                                                                                                                                                   |                  | [opsem][]              | *n/a* |
| [libc 1.0 release readiness](https://rust-lang.github.io/rust-project-goals/2026/libc-1.0.html)                                                                                                         | [Yuki Okushi][]       | [crate-maintainers][]  | *n/a* |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2026/libtest-json.html)                                                                                      | [Ed Page][]           | [cargo][]              | *n/a* |
|                                                                                                                                                   |                  | [libs-api][]           | *n/a* |
|                                                                                                                                                   |                  | [testing-devex][]      | *n/a* |
| [Implement Open Rust Namespace Support](https://rust-lang.github.io/rust-project-goals/2026/open-namespaces.html)                                                                                       | ![Help Wanted][] | [cargo][]              | *n/a* |
|                                                                                                                                                   |                  | [compiler][]           | *n/a* |
| [Establish a Spot for Safety-Critical Lints in Clippy](https://rust-lang.github.io/rust-project-goals/2026/safety-critical-lints-in-clippy.html)                                                        | [Pete LeVasseur][]      | [clippy][]             | *n/a* |
| [Stabilize never type (`!`)](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html)                                                                                             | [waffle][]    | [lang][]               | *n/a* |
|                                                                                                                                                   |                  | [types][]              | *n/a* |
| [Stabilizing `f16`](https://rust-lang.github.io/rust-project-goals/2026/stabilizing-f16.html)                                                                                                           | [Folkert de Vries][]      | [compiler][]           | *n/a* |
|                                                                                                                                                   |                  | [lang][]               | *n/a* |
|                                                                                                                                                   |                  | [libs-api][]           | *n/a* |
| [Type System Documentation](https://rust-lang.github.io/rust-project-goals/2026/typesystem-docs.html)                                                                                                   | [Boxy][]         | [types][]              | *n/a* |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                                                                       | [Jack Wrenn][]         | [book][]               | *n/a* |
|                                                                                                                                                   |                  | [clippy][]             | *n/a* |
|                                                                                                                                                   |                  | [lang][]               | *n/a* |
|                                                                                                                                                   |                  | [libs][]               | *n/a* |
|                                                                                                                                                   |                  | [rustdoc][]            | *n/a* |
|                                                                                                                                                   |                  | [rustfmt][]            | *n/a* |
|                                                                                                                                                   |                  | [spec][]               | *n/a* |
|                                                                                                                                                   |                  | [style][]              | *n/a* |
| [Establish a User Research Team](https://rust-lang.github.io/rust-project-goals/2026/user-research-team.html)                                                                                           | [Niko Matsakis][]    | [leadership-council][] | *n/a* |


### Goals by champion

| Champion          | # | Goal                                                                                                                                              |
| :--               | :-- | :--                                                                                                                                               |
| [Amanieu d'Antras][]          | 4 | [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                     |
|                   |   | [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html)                                                                                                   |
|                   |   | [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                       |
|                   |   | [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                                                                                       |
| [Boxy][]          | 2 | [Assumptions on Binders](https://rust-lang.github.io/rust-project-goals/2026/assumptions_on_binders.html)                                                                                               |
|                   |   | [Full Const Generics](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html)                                                                                                          |
| [Crystal Durham][]            | 1 | [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                              |
| [Alice Ryhl][]         | 1 | [In-place initialization](https://rust-lang.github.io/rust-project-goals/2026/in-place-init.html)                                                                                                       |
| [Guillaume Gomez][]   | 1 | [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                     |
| [Adam Harvey][]        | 1 | [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                                                                          |
| [Mark Rousskov][]  | 1 | [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                                                                          |
| [@Nadrieril][]        | 1 | [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                              |
| [Pete LeVasseur][]       | 1 | [Stabilize FLS Release Cadence](https://rust-lang.github.io/rust-project-goals/2026/stabilize-fls-releases.html)                                                                                        |
| [Ralf Jung][]         | 3 | [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                                                                             |
|                   |   | [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html)                                                                                                   |
|                   |   | [Normative Documentation for Sound `unsafe` Rust](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html)                                                             |
| [Takayuki Maeda][]          | 1 | [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html)                                                                                                  |
| [Wesley Wiser][]      | 2 | [Stabilize Rust for Linux compiler features](https://rust-lang.github.io/rust-project-goals/2026/rust-for-linux-compiler-features.html)                                                                 |
|                   |   | [Wasm Components](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html)                                                                                                             |
| [Alona Enraght-Moony][]    | 1 | [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html)                                                |
| [Arlo Siemsen][]           | 1 | [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                                                                          |
| [bjorn3][]           | 1 | [Improve `rustc_codegen_cranelift` performance](https://rust-lang.github.io/rust-project-goals/2026/improve-cg_clif-performance.html)                                                                   |
| [Boxy][]          | 1 | [Type System Documentation](https://rust-lang.github.io/rust-project-goals/2026/typesystem-docs.html)                                                                                                   |
| [Connor Horman][]      | 1 | [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                       |
| [Taylor Cramer][]         | 1 | [Implement Supertrait `auto impl`](https://rust-lang.github.io/rust-project-goals/2026/supertrait-auto-impl.html)                                                                                       |
| [David Wood][]        | 3 | [AArch64 Pointer Authentication using aarch64-unknown-linux-pauthtest target on Linux ELF platforms](https://rust-lang.github.io/rust-project-goals/2026/aarch64_pointer_authentication_pauthtest.html) |
|                   |   | [Implement and Maintain MC/DC Coverage Support](https://rust-lang.github.io/rust-project-goals/2026/mcdc-coverage-support.html)                                                                         |
|                   |   | [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                       |
| [dianne][]           | 1 | [Redesigning `super let`: Flexible Temporary Lifetime Extension](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html)                                                        |
| [Mario Carneiro][]          | 1 | [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                                                                       |
| [Ding Xiang Fei][] | 1 | [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                                                                         |
| [Dirkjan Ochtman][]              | 1 | [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                                                                          |
| [David Tolnay][]          | 1 | [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                                                                                  |
| [Eric Holk][]            | 1 | [Async statemachine optimisation](https://rust-lang.github.io/rust-project-goals/2026/async-statemachine-optimisation.html)                                                                             |
| [Eric Huss][]            | 2 | [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                     |
|                   |   | [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html)                                                                                                                         |
| [Ed Page][]            | 2 | [Cargo cross workspace cache](https://rust-lang.github.io/rust-project-goals/2026/cargo-cross-workspace-cache.html)                                                                                     |
|                   |   | [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html)                                                |
| [Jack Huey][]         | 7 | [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                                                                                   |
|                   |   | [Case study for experimental language specification, with integration into project teams and processes](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html)   |
|                   |   | [Expanding a-mir-formality to work better as a Rust type system spec](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html)                                                         |
|                   |   | [Immobile types and guaranteed destructors](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html)                                                                                        |
|                   |   | [Incremental Systems Rethought](https://rust-lang.github.io/rust-project-goals/2026/incremental-system-rethought.html)                                                                                  |
|                   |   | [Stabilize and model Polonius Alpha](https://rust-lang.github.io/rust-project-goals/2026/polonius.html)                                                                                                 |
|                   |   | [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                                                                       |
| [Josh Triplett][]     | 3 | [Case study for experimental language specification, with integration into project teams and processes](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html)   |
|                   |   | [Declarative (`macro_rules!`) macro improvements](https://rust-lang.github.io/rust-project-goals/2026/macro-improvements.html)                                                                          |
|                   |   | [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                                                                             |
| [lcnr][]             | 5 | [Dictionary Passing Style Experiment](https://rust-lang.github.io/rust-project-goals/2026/dictionary-passing-style-experiment.html)                                                                     |
|                   |   | [Immobile types and guaranteed destructors](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html)                                                                                        |
|                   |   | [Prepare TAIT + RTN for stabilization](https://rust-lang.github.io/rust-project-goals/2026/rtn.html)                                                                                                    |
|                   |   | [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                       |
|                   |   | [Stabilize the next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html)                                                                                      |
| [Rémy Rakic][]              | 1 | [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                                                                         |
| [Mads Marquart][]          | 1 | [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                       |
| [Niko Matsakis][]     | 6 | [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html)                                                                                                  |
|                   |   | [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html)                                                                                                         |
|                   |   | [Full Const Generics](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html)                                                                                                          |
|                   |   | [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                       |
|                   |   | [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                                                                       |
|                   |   | [Stabilize the next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html)                                                                                      |
| [Oliver Scherer][]          | 7 | [Architectural groundwork for expansion-time evaluation](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html)                                                            |
|                   |   | [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                                                                                  |
|                   |   | [Const Traits](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html)                                                                                                                   |
|                   |   | [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2026/pin-ergonomics.html)                                                                                 |
|                   |   | [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                              |
|                   |   | [Project goal - High-Level ML optimizations](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html)                                                                                    |
|                   |   | [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                                                                             |
| [Vadim Petrochenkov][]     | 3 | [Architectural groundwork for expansion-time evaluation](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html)                                                            |
|                   |   | [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2026/parallel-front-end.html)                                                                                             |
|                   |   | [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2026/pub-priv.html)                                                                                              |
| [Ramon de C Valle][]          | 1 | [Stabilize MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2026/stabilization-of-sanitizer-support.html)                                                    |
| [Scott McMurray][]         | 3 | [Explicit tail calls & `loop_match`](https://rust-lang.github.io/rust-project-goals/2026/tail-call-loop-match.html)                                                                                     |
|                   |   | [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                       |
|                   |   | [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                                                                             |
| [Tyler Mandry][]          | 9 | [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                                                                                   |
|                   |   | [Async Future Memory Optimisation](https://rust-lang.github.io/rust-project-goals/2026/async-future-memory-optimisation.html)                                                                           |
|                   |   | [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                                                                             |
|                   |   | [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                                                                                  |
|                   |   | [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                                                                         |
|                   |   | [Nightly support for function overloading in FFI bindings](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html)                                                                |
|                   |   | [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html)                                                                                                             |
|                   |   | [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                                                                       |
|                   |   | [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                                                                                       |
| [TC][]      | 6 | [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                                                                                   |
|                   |   | [Const Traits](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html)                                                                                                                   |
|                   |   | [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2026/pin-ergonomics.html)                                                                                 |
|                   |   | [Prepare TAIT + RTN for stabilization](https://rust-lang.github.io/rust-project-goals/2026/rtn.html)                                                                                                    |
|                   |   | [Project goal - High-Level ML optimizations](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html)                                                                                    |
|                   |   | [Redesigning `super let`: Flexible Temporary Lifetime Extension](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html)                                                        |
| [Weihang Lo][]        | 1 | [Stabilize Cargo SBOM precursor](https://rust-lang.github.io/rust-project-goals/2026/stabilize-cargo-sbom.html)                                                                                         |
| [Jane Lusby][]            | 1 | [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                     |


### Goals by team

The following table highlights the support level requested from each affected team. Each goal specifies the level of involvement needed:

* **Small**: The team only needs to do routine activities (e.g., reviewing a few small PRs).
* **Medium**: Dedicated support from one team member, but the rest of the team doesn't need to be heavily involved.
* **Large**: Deeper review and involvement from the entire team (e.g., design meetings, complex RFCs).

"Small" asks require someone on the team to "second" the goal. "Medium" and "Large" asks require a dedicated champion from the team.


#### book team
| Goal                                        | Level | Champion | Notes |
| :--                                         | :--   | :-- | :-- |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html) | Small |  | \*1 |


\*1: Will need approval for book changes. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))

#### bootstrap team
| Goal                                                                                           | Level | Champion | Notes |
| :--                                                                                            | :--   | :-- | :-- |
| [Stabilize MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2026/stabilization-of-sanitizer-support.html) | Small |  |  |

#### cargo team
| Goal                                                                                                | Level  | Champion   | Notes                 |
| :--                                                                                                 | :--    | :--        | :--                   |
| [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html)                                                                           | Large  | [Eric Huss][]     | \*1                   |
| [Stabilize Cargo SBOM precursor](https://rust-lang.github.io/rust-project-goals/2026/stabilize-cargo-sbom.html)                                           | Medium | [Weihang Lo][] |                       |
| [Cargo cross workspace cache](https://rust-lang.github.io/rust-project-goals/2026/cargo-cross-workspace-cache.html)                                       | Medium | [Ed Page][]     | \*2                   |
| [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                            | Medium | [Arlo Siemsen][]    | \*3                   |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html)                                                           | Small  |            | Stabilization process |
| [Stabilize Cargo's linting system](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html)                                                  | Small  |            | \*4                   |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2026/libtest-json.html)                                        | Small  |            |                       |
| [Improve `rustc_codegen_cranelift` performance](https://rust-lang.github.io/rust-project-goals/2026/improve-cg_clif-performance.html)                     | Small  |            | \*5                   |
| [Implement Open Rust Namespace Support](https://rust-lang.github.io/rust-project-goals/2026/open-namespaces.html)                                         | Small  |            |                       |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2026/pub-priv.html)                                                | Small  |            |                       |
| [Prototype a new set of Cargo "plumbing" commands](https://rust-lang.github.io/rust-project-goals/2026/cargo-plumbing.html)                               | Small  |            | \*6                   |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html)  | Small  | [Ed Page][]     | \*7                   |
| [Interactive cargo-tree: TUI for Cargo's dependency graph visualization](https://rust-lang.github.io/rust-project-goals/2026/interactive-cargo-tree.html) | Small  |            | \*8                   |


\*1: Reviews of [rust-lang/rfcs#3874](https://github.com/rust-lang/rfcs/issues/3874) and [rust-lang/rfcs#3875](https://github.com/rust-lang/rfcs/issues/3875) and many implementation patches ([from here](https://rust-lang.github.io/rust-project-goals/2026/build-std.html))


\*2: Design and code reviews ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-cross-workspace-cache.html))


\*3: Support needed for registry field design and resolver consistency. ([from here](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html))


\*4: Code reviews and maybe a design discussion or two ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html))


\*5: In case we end up pursuing JITing as a way to improve performance that will eventually need native integration with `cargo run`. For now we're just prototyping, and so the occasional vibe check should be sufficient ([from here](https://rust-lang.github.io/rust-project-goals/2026/improve-cg_clif-performance.html))


\*6: PR reviews for Cargo changes; design discussions ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-plumbing.html))


\*7: Discussion and moral support ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html))


\*8: Alignment on direction, possible integration help and review. ([from here](https://rust-lang.github.io/rust-project-goals/2026/interactive-cargo-tree.html))

#### clippy team
| Goal                                                                                       | Level | Champion | Notes |
| :--                                                                                        | :--   | :-- | :-- |
| [Stabilize Cargo's linting system](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html)                                         | Small |  | \*1 |
| [Establish a Spot for Safety-Critical Lints in Clippy](https://rust-lang.github.io/rust-project-goals/2026/safety-critical-lints-in-clippy.html) | Small |  | \*2 |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                | Small |  | \*3 |


\*1: Review our initial batch of lints to ensure they provide an example of adapting the existing lint guidelines to Cargo ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html))


\*2: Initial onboarding support for SCRC contributors; guidance on lint design ([from here](https://rust-lang.github.io/rust-project-goals/2026/safety-critical-lints-in-clippy.html))


\*3: Will need approval for clippy support. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))

#### compiler team
| Goal                                                                                                                                              | Level  | Champion          | Notes                  |
| :--                                                                                                                                               | :--    | :--               | :--                    |
| [Architectural groundwork for expansion-time evaluation](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html)                                                            | Large  | [Vadim Petrochenkov][]     | \*1                    |
| [Async Future Memory Optimisation](https://rust-lang.github.io/rust-project-goals/2026/async-future-memory-optimisation.html)                                                                           | Medium | [Tyler Mandry][]          |                        |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                                                                             | Medium | [Oliver Scherer][]          | Standard reviews       |
| [Project goal - High-Level ML optimizations](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html)                                                                                    | Medium | [Oliver Scherer][]          | \*2                    |
| [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                              | Medium | [Oliver Scherer][]          | \*3                    |
| [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html)                                                                                                  | Medium | [Takayuki Maeda][]          | Implementation review  |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                                                                         | Medium | [Ding Xiang Fei][] | \*4                    |
| [Wasm Components](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html)                                                                                                             | Medium | [Wesley Wiser][]      | \*5                    |
| [Stabilize MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2026/stabilization-of-sanitizer-support.html)                                                    | Medium | [Ramon de C Valle][]          | Reviews, stabilization |
| [Incremental Systems Rethought](https://rust-lang.github.io/rust-project-goals/2026/incremental-system-rethought.html)                                                                                  | Medium | [Jack Huey][]         |                        |
| [Improve `rustc_codegen_cranelift` performance](https://rust-lang.github.io/rust-project-goals/2026/improve-cg_clif-performance.html)                                                                   | Medium | [bjorn3][]           | \*6                    |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                     | Medium | [Jane Lusby][]            | \*7                    |
| [Redesigning `super let`: Flexible Temporary Lifetime Extension](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html)                                                        | Medium | [dianne][]           |                        |
| [Implement and Maintain MC/DC Coverage Support](https://rust-lang.github.io/rust-project-goals/2026/mcdc-coverage-support.html)                                                                         | Medium | [David Wood][]        | \*8                    |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html)                                                                                                   | Medium |                   | RFC decision           |
| [Async statemachine optimisation](https://rust-lang.github.io/rust-project-goals/2026/async-statemachine-optimisation.html)                                                                             | Medium | [Eric Holk][]            | \*9                    |
| [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                                                                             | Medium | [Ralf Jung][]         | \*10                   |
| [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                       | Medium | [Mads Marquart][]          | Implementation reviews |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2026/pub-priv.html)                                                                                              | Medium | [Vadim Petrochenkov][]     | \*11                   |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2026/pin-ergonomics.html)                                                                                 | Medium | [Oliver Scherer][]          | Reviews                |
| [Stabilize Rust for Linux compiler features](https://rust-lang.github.io/rust-project-goals/2026/rust-for-linux-compiler-features.html)                                                                 | Medium | [Wesley Wiser][]      | Reviews, RfL meetings  |
| [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                       | Medium | [David Wood][]        | \*12                   |
| [Const Traits](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html)                                                                                                                   | Small  |                   | Code reviews           |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html)                                                                                                         | Small  |                   | \*13                   |
| [Stabilize Cargo's linting system](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html)                                                                                                | Small  |                   | \*14                   |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html)                                                                                                             | Small  |                   | \*15                   |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2026/parallel-front-end.html)                                                                                             | Small  |                   | Code Reviews           |
| [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                                                                                       | Small  |                   |                        |
| [Implement Open Rust Namespace Support](https://rust-lang.github.io/rust-project-goals/2026/open-namespaces.html)                                                                                       | Small  |                   | \*16                   |
| [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                                                                       | Small  |                   |                        |
| [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html)                                                                                                                         | Small  |                   | \*17                   |
| [Stabilizing `f16`](https://rust-lang.github.io/rust-project-goals/2026/stabilizing-f16.html)                                                                                                           | Small  |                   |                        |
| [Nightly support for function overloading in FFI bindings](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html)                                                                | Small  |                   | \*18                   |
| [AArch64 Pointer Authentication using aarch64-unknown-linux-pauthtest target on Linux ELF platforms](https://rust-lang.github.io/rust-project-goals/2026/aarch64_pointer_authentication_pauthtest.html) | Small  | [David Wood][]        | \*19                   |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html)                                                                                                         | Small  |                   | Reviews                |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                                                                                  | Small  | [Oliver Scherer][]          | Reviews                |
| [Explicit tail calls & `loop_match`](https://rust-lang.github.io/rust-project-goals/2026/tail-call-loop-match.html)                                                                                     | Small  |                   | \*20                   |


\*1: Significant refactoring of the resolver, reviews from [Vadim Petrochenkov][] ([from here](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html))


\*2: My changes should be contained to few places in the compiler. Potentially one frontend macro/intrinsic, and otherwise almost exclusively in the backend. ([from here](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html))


\*3: Implementation reviews ([Oliver Scherer][] will review Proposal 2) ([from here](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html))


\*4: Reviews of big changes needed; also looking for implementation help ([from here](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html))


\*5: Targets are small but `async fn` is not ([from here](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html))


\*6: Depending on what ways we end up pursuing, we might need no rustc side changes at all or medium sized changes. ([from here](https://rust-lang.github.io/rust-project-goals/2026/improve-cg_clif-performance.html))


\*7: Design discussions and implementation review. ([from here](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html))


\*8: Review of implementation PRs; guidance on architecture to avoid previous maintenance issues ([from here](https://rust-lang.github.io/rust-project-goals/2026/mcdc-coverage-support.html))


\*9: Most will be review work, but pushing optimisations to the max will possibly touch on some controversial points that need discussion ([from here](https://rust-lang.github.io/rust-project-goals/2026/async-statemachine-optimisation.html))


\*10: Champion: [Ralf Jung][]. Design discussions, PR review, and upstream integration. ([from here](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html))


\*11: Design discussions, PR review ([from here](https://rust-lang.github.io/rust-project-goals/2026/pub-priv.html))


\*12: Standard reviews for stabilization and SVE work ([from here](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html))


\*13: Reviewing any further compiler changes ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html))


\*14: Review our initial batch of lints to ensure they provide an example of adapting the existing lint guidelines to Cargo ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-lints.html))


\*15: Standard reviews for trait implementation PRs ([from here](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html))


\*16: Design discussions, PR review ([from here](https://rust-lang.github.io/rust-project-goals/2026/open-namespaces.html))


\*17: Reviews of [rust-lang/rfcs#3874](https://github.com/rust-lang/rfcs/issues/3874) and [rust-lang/rfcs#3875](https://github.com/rust-lang/rfcs/issues/3875) and any implementation patches ([from here](https://rust-lang.github.io/rust-project-goals/2026/build-std.html))


\*18: Most complexity is in the type system ([from here](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html))


\*19: Design discussions, PR review ([from here](https://rust-lang.github.io/rust-project-goals/2026/aarch64_pointer_authentication_pauthtest.html))


\*20: We expect to only need normal reviews. ([from here](https://rust-lang.github.io/rust-project-goals/2026/tail-call-loop-match.html))

#### crate-maintainers team
| Goal                                      | Level | Champion | Notes |
| :--                                       | :--   | :-- | :-- |
| [libc 1.0 release readiness](https://rust-lang.github.io/rust-project-goals/2026/libc-1.0.html) | Small |  |  |

#### crates-io team
| Goal                                                     | Level | Champion   | Notes |
| :--                                                      | :--   | :--        | :-- |
| [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html)                                | Small |            | \*1 |
| [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html) | Small | [Adam Harvey][] | \*2 |


\*1: Reviews of [rust-lang/rfcs#3874](https://github.com/rust-lang/rfcs/issues/3874) and [rust-lang/rfcs#3875](https://github.com/rust-lang/rfcs/issues/3875) and any implementation patches ([from here](https://rust-lang.github.io/rust-project-goals/2026/build-std.html))


\*2: Primarily focused on potential future logging/bandwidth savings. ([from here](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html))

#### edition team
| Goal                                                                          | Level | Champion | Notes |
| :--                                                                           | :--   | :--    | :-- |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html) | Large | [Eric Huss][] | \*1 |


\*1: Review the feasibility of this proposal as well as the specific API changes. ([from here](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html))

#### fls team
| Goal                                                       | Level | Champion    | Notes |
| :--                                                        | :--   | :--         | :-- |
| [Stabilize FLS Release Cadence](https://rust-lang.github.io/rust-project-goals/2026/stabilize-fls-releases.html) | Large | [Pete LeVasseur][] | \*1 |


\*1: Core work of authoring and releasing FLS versions on schedule ([from here](https://rust-lang.github.io/rust-project-goals/2026/stabilize-fls-releases.html))

#### infra team
| Goal                                                                                           | Level  | Champion         | Notes                 |
| :--                                                                                            | :--    | :--              | :--                   |
| [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html)                                       | Medium | [Mark Rousskov][] | \*1                   |
| [Project goal - High-Level ML optimizations](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html)                                 | Small  |                  | \*2                   |
| [Stabilize MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2026/stabilization-of-sanitizer-support.html) | Small  |                  |                       |
| [Implement and Maintain MC/DC Coverage Support](https://rust-lang.github.io/rust-project-goals/2026/mcdc-coverage-support.html)                      | Small  |                  | \*3                   |
| [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                          | Small  |                  | Upstream integration. |


\*1: Critical for setting up the signing pipeline and Azure deployment. ([from here](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html))


\*2: I will work with [Jakub Beránek][] to add more bootstrap options to build and configure MLIR (an LLVM subproject) ([from here](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html))


\*3: CI support for MC/DC testing ([from here](https://rust-lang.github.io/rust-project-goals/2026/mcdc-coverage-support.html))

#### lang team
| Goal                                                                                                                                            | Level  | Champion      | Notes           |
| :--                                                                                                                                             | :--    | :--           | :--             |
| [Const Traits](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html)                                                                                                                 | Large  | [TC][]  | \*1             |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                                                                           | Large  | [Scott McMurray][]     | \*2             |
| [Full Const Generics](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html)                                                                                                        | Large  | [Niko Matsakis][] | \*3             |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                                                                       | Large  | [Tyler Mandry][]      | \*4             |
| [Redesigning `super let`: Flexible Temporary Lifetime Extension](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html)                                                      | Large  | [TC][]  | \*5             |
| [Immobile types and guaranteed destructors](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html)                                                                                      | Large  | [Jack Huey][]     | \*6             |
| [In-place initialization](https://rust-lang.github.io/rust-project-goals/2026/in-place-init.html)                                                                                                     | Medium | [Alice Ryhl][]     | \*7             |
| [Stabilize the next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html)                                                                                    | Medium | [Niko Matsakis][] | \*8             |
| [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                                                                                 | Medium | [Tyler Mandry][]      | \*9             |
| [Project goal - High-Level ML optimizations](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html)                                                                                  | Medium | [TC][]  | \*10            |
| [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                            | Medium | [@Nadrieril][]    | \*11            |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html)                                                                                                           | Medium | [Tyler Mandry][]      | \*12            |
| [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html)                                                                                                | Medium | [Niko Matsakis][] | RFC decision    |
| [Declarative (`macro_rules!`) macro improvements](https://rust-lang.github.io/rust-project-goals/2026/macro-improvements.html)                                                                        | Medium | [Josh Triplett][] | \*13            |
| [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                                                                                     | Medium | [Tyler Mandry][]      |                 |
| [Implement Supertrait `auto impl`](https://rust-lang.github.io/rust-project-goals/2026/supertrait-auto-impl.html)                                                                                     | Medium | [Taylor Cramer][]     | \*14            |
| [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                                                                     | Medium | [Tyler Mandry][]      | \*15            |
| [Prepare TAIT + RTN for stabilization](https://rust-lang.github.io/rust-project-goals/2026/rtn.html)                                                                                                  | Medium | [TC][]  | \*16            |
| [Nightly support for function overloading in FFI bindings](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html)                                                              | Medium | [Tyler Mandry][]      | \*17            |
| [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                     | Medium | [Scott McMurray][]     | \*18            |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html)                                                                                                       | Medium | [Niko Matsakis][] |                 |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2026/pin-ergonomics.html)                                                                               | Medium | [TC][]  | Design meeting? |
| [Explicit tail calls & `loop_match`](https://rust-lang.github.io/rust-project-goals/2026/tail-call-loop-match.html)                                                                                   | Medium | [Scott McMurray][]     | \*19            |
| [Case study for experimental language specification, with integration into project teams and processes](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html) | Medium | [Josh Triplett][] |                 |
| [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                     | Medium | [Niko Matsakis][] | \*20            |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html)                                                                                                       | Small  |               | \*21            |
| [Normative Documentation for Sound `unsafe` Rust](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html)                                                           | Small  |               | \*22            |
| [Wasm Components](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html)                                                                                                           | Small  |               | \*23            |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                   | Small  |               | \*24            |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html)                                                                                                 | Small  | [Amanieu d'Antras][]      | RFC decision    |
| [Stabilizing `f16`](https://rust-lang.github.io/rust-project-goals/2026/stabilizing-f16.html)                                                                                                         | Small  |               | \*25            |
| [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                                                                           | Small  | [Tyler Mandry][]      | \*26            |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                                                                     | Small  | [Niko Matsakis][] | \*27            |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                                                                                | Small  | [Tyler Mandry][]      | Reviews         |
| [Stabilize never type (`!`)](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html)                                                                                           | Small  |               | \*28            |


\*1: Semantics, syntax, and stabilization decisions ([from here](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html))


\*2: Design meeting, experiment ([from here](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html))


\*3: Stabilization decisions, directional alignment ([from here](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html))


\*4: Aiming for two design meetings; large language feature ([from here](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html))


\*5: Would need a design meeting and RFC review. ([from here](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html))


\*6: Design session needed to work through design ([from here](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html))


\*7: Review and accept a design space RFC ([from here](https://rust-lang.github.io/rust-project-goals/2026/in-place-init.html))


\*8: Stabilization decision for user facing changes ([from here](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html))


\*9: Reviews, Lang/RfL meetings ([from here](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html))


\*10: Discussions to understand which parts of gpu programming and `std::offload` are problematic wrt. stabilization, from a lang perspective. Non-blocking, since we are not rushing stabilization. ([from here](https://rust-lang.github.io/rust-project-goals/2026/high-level-ml.html))


\*11: Vibe check and RFC review ([from here](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html))


\*12: Continued experiment support, design feedback ([from here](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html))


\*13: This is a stabilization, but we have previously explored the design in detail, and it's simple and straightforward. It should be able to take place asynchronously. Nonetheless, I can upgrade this to "Large" if people believe it rises to that level. ([from here](https://rust-lang.github.io/rust-project-goals/2026/macro-improvements.html))


\*14: Team aligned already on the shape of the feature ([from here](https://rust-lang.github.io/rust-project-goals/2026/supertrait-auto-impl.html))


\*15: Resolve design concerns like `#[override]` and review stabilization ([from here](https://rust-lang.github.io/rust-project-goals/2026/specialization.html))


\*16: RFC review, design discussions ([from here](https://rust-lang.github.io/rust-project-goals/2026/rtn.html))


\*17: Design meeting Experiment ([from here](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html))


\*18: Champion and (ideally) a lang meeting ([from here](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html))


\*19: Some architectures cannot support guaranteed tail calls. Our current list of limitations is:<br><br>- `wasm32`/`wasm64` need the `tail-call` target feature to be enabled<br>- `powerpc` (when `elf1` is used) cannot tail call functions in other objects<br><br>Hence, rust code using guaranteed tail calls is not as portable as standard rust code. We need T-lang feedback on how to resolve this.<br><br>The all-hands is well-timed to figure out a solution. ([from here](https://rust-lang.github.io/rust-project-goals/2026/tail-call-loop-match.html))


\*20: RFC decision for [rfcs#3838], stabilization sign-off ([from here](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html))


\*21: Stabilization discussions ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html))


\*22: Feedback on language semantics questions as needed ([from here](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html))


\*23: Experimentation with native Wasm features will need approval. May become "medium" if we are somehow really successful. ([from here](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html))


\*24: Review of the feature and lang implications. ([from here](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html))


\*25: occasionally being fast-tracked would be nice ([from here](https://rust-lang.github.io/rust-project-goals/2026/stabilizing-f16.html))


\*26: Champion: [Tyler Mandry][]. General support and guidance. ([from here](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html))


\*27: Will need approval for stabilization. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))


\*28: Most of the plans / design was already approved, only minor sign-offs required ([from here](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html))

#### lang-docs team
| Goal                                                                                      | Level  | Champion     | Notes |
| :--                                                                                       | :--    | :--          | :-- |
| [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                           | Medium | [TC][] | \*1 |
| [Normative Documentation for Sound `unsafe` Rust](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html)     | Small  |              | \*2 |
| [Expanding a-mir-formality to work better as a Rust type system spec](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html) | Small  |              | \*3 |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html)                                                 | Small  |              |     |


\*1: Reviews, Lang/RfL meetings ([from here](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html))


\*2: Standard PR reviews for Rust Reference ([from here](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html))


\*3: General discussion of shape of integration of a-mir-formality into reference ([from here](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html))

#### leadership-council team
| Goal                                                    | Level | Champion | Notes |
| :--                                                     | :--   | :-- | :-- |
| [Establish a User Research Team](https://rust-lang.github.io/rust-project-goals/2026/user-research-team.html) | Small |  | \*1 |


\*1: Org decision to establish team, ongoing coordination ([from here](https://rust-lang.github.io/rust-project-goals/2026/user-research-team.html))

#### libs team
| Goal                                                                                       | Level | Champion | Notes                 |
| :--                                                                                        | :--   | :-- | :--                   |
| [Improving Unsafe Code Documentation in the Rust Standard Library](https://rust-lang.github.io/rust-project-goals/2026/improve-std-unsafe.html)  | Small |  | Review pull requests; |
| [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                            | Small |  | Reviews               |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                  | Small |  | \*1                   |
| [Wasm Components](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html)                                                      | Small |  | \*2                   |
| [Redesigning `super let`: Flexible Temporary Lifetime Extension](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html) | Small |  | \*3                   |
| [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                | Small |  |                       |
| [build-std](https://rust-lang.github.io/rust-project-goals/2026/build-std.html)                                                                  | Small |  | \*4                   |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                | Small |  | \*5                   |
| [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                | Small |  | Changes to `derive`   |


\*1: Small reviews of library PRs (implementing FP for core & std types) ([from here](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html))


\*2: Threading support will need review ([from here](https://rust-lang.github.io/rust-project-goals/2026/wasm-components.html))


\*3: Since `super let` affects the standard library, the library team should be on-board with any new directions it takes. Additionally, library team review may be required for changes to `pin!`'s implementation. ([from here](https://rust-lang.github.io/rust-project-goals/2026/redesigning-super-let.html))


\*4: Reviews of [rust-lang/rfcs#3874](https://github.com/rust-lang/rfcs/issues/3874) and [rust-lang/rfcs#3875](https://github.com/rust-lang/rfcs/issues/3875) and any implementation patches ([from here](https://rust-lang.github.io/rust-project-goals/2026/build-std.html))


\*5: Will need approval for documentation changes. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))

#### libs-api team
| Goal                                                                                  | Level  | Champion      | Notes          |
| :--                                                                                   | :--    | :--           | :--            |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)         | Large  | [Amanieu d'Antras][]      | \*1            |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                 | Medium | [Josh Triplett][] | Reviews        |
| [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                           | Medium | [Amanieu d'Antras][]      |                |
| [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                           | Medium | [Amanieu d'Antras][]      | \*2            |
| [Normative Documentation for Sound `unsafe` Rust](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html) | Small  |               | \*3            |
| [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                       | Small  |               | Stabilizations |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2026/libtest-json.html)                          | Small  |               |                |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                             | Small  |               | Reviews of RFC |
| [Stabilizing `f16`](https://rust-lang.github.io/rust-project-goals/2026/stabilizing-f16.html)                                               | Small  |               |                |
| [Nightly support for function overloading in FFI bindings](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html)    | Small  |               | \*4            |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html)                                             | Small  |               | \*5            |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                      | Small  | [David Tolnay][]      | Reviews        |


\*1: Determine what API changes should be made across editions. ([from here](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html))


\*2: Review RFC; review and approve stdarch SVE APIs ([from here](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html))


\*3: PR reviews for core/std public documentation; feedback on approach. ([from here](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html))


\*4: Would like to know if they have use cases for overloading in standard Rust, or if there are certain approaches they would like better. May be involved if experiment involves library surface area (e.g. `Fn` traits) ([from here](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html))


\*5: Reviews of RFC and API surface area ([from here](https://rust-lang.github.io/rust-project-goals/2026/ergonomic-rc.html))

#### opsem team
| Goal                                                                                      | Level  | Champion     | Notes                |
| :--                                                                                       | :--    | :--          | :--                  |
| [Normative Documentation for Sound `unsafe` Rust](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html)     | Large  | [Ralf Jung][]    | \*1                  |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html)                                           | Large  | [Ralf Jung][]    | Design meeting       |
| [BorrowSanitizer](https://rust-lang.github.io/rust-project-goals/2026/borrowsanitizer.html)                                                     | Medium | [Ralf Jung][]    | Champion: [Ralf Jung][]. |
| [Improving Unsafe Code Documentation in the Rust Standard Library](https://rust-lang.github.io/rust-project-goals/2026/improve-std-unsafe.html) | Small  |              | \*2                  |
| [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                      | Small  | [Crystal Durham][]       |                      |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                 | Small  |              | \*3                  |
| [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                               | Small  |              |                      |
| [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                               | Small  | [Connor Horman][] | \*4                  |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html)                          | Small  |              | \*5                  |


\*1: Review unsafe patterns, establish safety contracts, guide documentation ([from here](https://rust-lang.github.io/rust-project-goals/2026/safe-unsafe-for-safety-critical.html))


\*2: Review pull requests; answer questions on Zulip when there are different opinions about specific rules ([from here](https://rust-lang.github.io/rust-project-goals/2026/improve-std-unsafe.html))


\*3: Small reviews of RFC and/or compiler PRs ([from here](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html))


\*4: Doc changes if necessary ([from here](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html))


\*5: Problem statement review ([from here](https://rust-lang.github.io/rust-project-goals/2026/interop-problem-map.html))

#### project-exploit-mitigations team
| Goal                                                                                           | Level  | Champion | Notes              |
| :--                                                                                            | :--    | :--      | :--                |
| [Stabilize MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2026/stabilization-of-sanitizer-support.html) | Medium | [Ramon de C Valle][] | Dedicated reviewer |

#### rustdoc team
| Goal                                                                                               | Level  | Champion        | Notes |
| :--                                                                                                | :--    | :--             | :-- |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                      | Medium | [Guillaume Gomez][] | \*1 |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html)                                                          | Small  |                 | \*2 |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                        | Small  |                 | \*3 |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html) | Small  | [Alona Enraght-Moony][]  | \*4 |


\*1: Figure out how such API changes should be presented in the API docs. ([from here](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html))


\*2: Design decision and PR review ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-script.html))


\*3: Will need approval for rustdoc support. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))


\*4: Discussion and moral support ([from here](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html))

#### rustfmt team
| Goal                                        | Level | Champion | Notes |
| :--                                         | :--   | :-- | :-- |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html) | Small |  | \*1 |


\*1: Will need approval for rustfmt support. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))

#### rustup team
| Goal                                                     | Level  | Champion | Notes |
| :--                                                      | :--    | :--  | :-- |
| [Implement Verifiable Mirroring Prototype](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html) | Medium | [Dirkjan Ochtman][] | \*1 |


\*1: Required for integrating the prototype into the primary toolchain installer. ([from here](https://rust-lang.github.io/rust-project-goals/2026/mirroring.html))

#### spec team
| Goal                                                                                                                                            | Level | Champion | Notes |
| :--                                                                                                                                             | :--   | :-- | :-- |
| [Stabilize FLS Release Cadence](https://rust-lang.github.io/rust-project-goals/2026/stabilize-fls-releases.html)                                                                                      | Small |  | \*1 |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html)                                                                                                     | Small |  | \*2 |
| [Expanding a-mir-formality to work better as a Rust type system spec](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html)                                                       | Small |  | \*3 |
| [Case study for experimental language specification, with integration into project teams and processes](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html) | Small |  | \*4 |


\*1: Alignment on release cadence goal ([from here](https://rust-lang.github.io/rust-project-goals/2026/stabilize-fls-releases.html))


\*2: Will need approval for reference changes. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))


\*3: General discussion of integration of a-mir-formality with reference ([from here](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html))


\*4: General discussion on how this may align with other efforts to specify Rust. ([from here](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html))

#### style team
| Goal                                        | Level | Champion | Notes |
| :--                                         | :--   | :-- | :-- |
| [Stabilize Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html) | Small |  | \*1 |


\*1: Will need approval for style guide changes. ([from here](https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html))

#### testing-devex team
| Goal                                                         | Level | Champion | Notes |
| :--                                                          | :--   | :-- | :-- |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2026/libtest-json.html) | Small |  | \*1 |


\*1: Design discussions and review ([from here](https://rust-lang.github.io/rust-project-goals/2026/libtest-json.html))

#### types team
| Goal                                                                                                                                            | Level  | Champion  | Notes                  |
| :--                                                                                                                                             | :--    | :--       | :--                    |
| [Const Traits](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html)                                                                                                                 | Large  | [Oliver Scherer][]  | \*1                    |
| [Stabilize the next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html)                                                                                    | Large  | [lcnr][]     | \*2                    |
| [Arbitrary Self Types](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html)                                                                                                 | Large  | [Jack Huey][] | \*3                    |
| [Full Const Generics](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html)                                                                                                        | Large  | [Boxy][]  | \*4                    |
| [Stabilize and model Polonius Alpha](https://rust-lang.github.io/rust-project-goals/2026/polonius.html)                                                                                               | Large  | [Jack Huey][] | \*5                    |
| [Stabilize concrete type specialization](https://rust-lang.github.io/rust-project-goals/2026/specialization.html)                                                                                     | Large  | [Jack Huey][] | \*6                    |
| [Immobile types and guaranteed destructors](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html)                                                                                      | Large  | [lcnr][]     | \*7                    |
| [Assumptions on Binders](https://rust-lang.github.io/rust-project-goals/2026/assumptions_on_binders.html)                                                                                             | Medium | [Boxy][]  | \*8                    |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html)                                                                                                       | Medium | [Rémy Rakic][]      | \*9                    |
| [Dictionary Passing Style Experiment](https://rust-lang.github.io/rust-project-goals/2026/dictionary-passing-style-experiment.html)                                                                   | Medium | [lcnr][]     | Review and discussions |
| [Prepare TAIT + RTN for stabilization](https://rust-lang.github.io/rust-project-goals/2026/rtn.html)                                                                                                  | Medium | [lcnr][]     | \*10                   |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2026/pin-ergonomics.html)                                                                               | Medium | [Oliver Scherer][]  | Reviews                |
| [Architectural groundwork for expansion-time evaluation](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html)                                                          | Medium | [Oliver Scherer][]  | \*11                   |
| [Case study for experimental language specification, with integration into project teams and processes](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html) | Medium | [Jack Huey][] | \*12                   |
| [Sized Hierarchy and Scalable Vectors](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html)                                                                                     | Medium | [lcnr][]     | \*13                   |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html)                                                                                           | Small  |           | \*14                   |
| [Control over Drop semantics](https://rust-lang.github.io/rust-project-goals/2026/manually-drop-attr.html)                                                                                            | Small  |           |                        |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html)                                                                                                           | Small  |           | \*15                   |
| [Box notation for dyn async trait](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html)                                                                                                | Small  |           | \*16                   |
| [Evolving the standard library API across editions](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html)                                                                   | Small  |           | \*17                   |
| [Stabilize the Try trait](https://rust-lang.github.io/rust-project-goals/2026/stabilize-try.html)                                                                                                     | Small  |           |                        |
| [Implement Supertrait `auto impl`](https://rust-lang.github.io/rust-project-goals/2026/supertrait-auto-impl.html)                                                                                     | Small  |           | \*18                   |
| [Nightly support for function overloading in FFI bindings](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html)                                                              | Small  |           | \*19                   |
| [Type System Documentation](https://rust-lang.github.io/rust-project-goals/2026/typesystem-docs.html)                                                                                                 | Small  | [Boxy][]  | \*20                   |
| [Open Enums](https://rust-lang.github.io/rust-project-goals/2026/open-enums.html)                                                                                                                     | Small  |           |                        |
| [Expanding a-mir-formality to work better as a Rust type system spec](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html)                                                       | Small  | [Jack Huey][] | \*21                   |
| [Stabilize never type (`!`)](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html)                                                                                           | Small  |           | \*22                   |


\*1: Implementation design and sign-off ([from here](https://rust-lang.github.io/rust-project-goals/2026/const-traits.html))


\*2: Stabilization decision, ongoing review work ([from here](https://rust-lang.github.io/rust-project-goals/2026/next-solver.html))


\*3: Review of type-system stabilization/implementation ([from here](https://rust-lang.github.io/rust-project-goals/2026/arbitrary-self-types.html))


\*4: a-mir-formality modeling, design alignment, reviews ([from here](https://rust-lang.github.io/rust-project-goals/2026/const-generics.html))


\*5: Design review, stabilization decision, reviews from [Jack Huey][] and [Matthew Jasper][] ([from here](https://rust-lang.github.io/rust-project-goals/2026/polonius.html))


\*6: Review future extensions for plausibility, soundness, and stabilization ([from here](https://rust-lang.github.io/rust-project-goals/2026/specialization.html))


\*7: Involved in implementation + review ([from here](https://rust-lang.github.io/rust-project-goals/2026/move-trait.html))


\*8: implementation/reviews/deciding on a design ([from here](https://rust-lang.github.io/rust-project-goals/2026/assumptions_on_binders.html))


\*9: Collaborating on a-mir-formality on the borrow checker integration; small reviews of RFC and/or compiler PRs ([from here](https://rust-lang.github.io/rust-project-goals/2026/field-projections.html))


\*10: Stabilization report review, TAIT interactions ([from here](https://rust-lang.github.io/rust-project-goals/2026/rtn.html))


\*11: Support for the restricted solver mode in the new solver ([from here](https://rust-lang.github.io/rust-project-goals/2026/expansion-time-evaluation.html))


\*12: Evaluate potential changes to (experimental) reference in routine team decisions ([from here](https://rust-lang.github.io/rust-project-goals/2026/experimental-language-specification.html))


\*13: Type System implementation and stabilization sign-off ([from here](https://rust-lang.github.io/rust-project-goals/2026/scalable-vectors.html))


\*14: General discussion on any additional type-system changes ([from here](https://rust-lang.github.io/rust-project-goals/2026/reflection-and-comptime.html))


\*15: Review work on the type system is expected to be trivial and feature-gated ([from here](https://rust-lang.github.io/rust-project-goals/2026/reborrow-traits.html))


\*16: May have changes to dyn-compatibility rules ([from here](https://rust-lang.github.io/rust-project-goals/2026/afidt-box.html))


\*17: Review of any changes to HIR ty lowering or method resolution ([from here](https://rust-lang.github.io/rust-project-goals/2026/library-api-evolution.html))


\*18: `r? types` when touching the type system. Expect that anything beyond "simple" types changes may be rejected or de-prioritized. [^types-small] ([from here](https://rust-lang.github.io/rust-project-goals/2026/supertrait-auto-impl.html))


\*19: No dedicated reviewer needed/given, but tracking issue should note the needed for dedicated types review prior to stabilization ([from here](https://rust-lang.github.io/rust-project-goals/2026/overloading-for-ffi.html))


\*20: Discussion and moral support ([from here](https://rust-lang.github.io/rust-project-goals/2026/typesystem-docs.html))


\*21: Members may have comments/thoughts on direction and priorities; Review work for a-mir-formality ([from here](https://rust-lang.github.io/rust-project-goals/2026/a-mir-formality.html))


\*22: We expect to only need normal reviews ([from here](https://rust-lang.github.io/rust-project-goals/2026/stabilize-never-type.html))

#### wg-mir-opt team
| Goal                                            | Level  | Champion | Notes          |
| :--                                             | :--    | :-- | :--            |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2026/mir-move-elimination.html) | Medium |  | Design meeting |

#### wg-parallel-rustc team
| Goal                                                  | Level | Champion      | Notes |
| :--                                                   | :--   | :--           | :-- |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2026/parallel-front-end.html) | Large | [Vadim Petrochenkov][] | \*1 |


\*1: Discussion and Implementation ([from here](https://rust-lang.github.io/rust-project-goals/2026/parallel-front-end.html))

# Frequently asked questions

## How does the goal process work?

**Project goals** are proposed bottom-up by a **point of contact**, somebody who is willing to commit resources (time, money, leadership) to seeing the work get done. The point of contact identifies the problem they want to address and sketches the solution of how they want to do so. They also identify the support they will need from the Rust teams (typically things like review bandwidth or feedback on RFCs). Teams then read the goals and provide feedback. If the goal is approved, teams are committing to support the point of contact in their work.

Project goals can vary in scope from an internal refactoring that affects only one team to a larger cross-cutting initiative. No matter its scope, accepting a goal should never be interpreted as a promise that the team will make any future decision (e.g., accepting an RFC that has yet to be written). Rather, it is a promise that the team are aligned on the contents of the goal thus far (including the design axioms and other notes) and will prioritize giving feedback and support as needed.

Of the proposed goals, a small subset are selected by the roadmap owner as **roadmap goals**. Roadmap goals are chosen for their high impact (many Rust users will be impacted) and their shovel-ready nature (the org is well-aligned around a concrete plan). Roadmap goals are the ones that will feature most prominently in our public messaging and which should be prioritized by Rust teams where needed.

## What goal were not accepted?

These goals were not accepted either for want of resources or consensus. In some cases, narrower versions of these goals were proposed instead.

| Goal                                                      | Point of contact | Task Owners and Champions |
| :--                                                       | :--      | :-- |
| [Crate Slicing for Faster Fresh Builds](https://rust-lang.github.io/rust-project-goals/2026/crate-slicing.html) | @yijunyu |  |


<!-- GitHub usernames -->

<!-- GitHub usernames -->

[@Nadrieril]: https://github.com/Nadrieril
[Aapo Alasuutari]: https://github.com/aapoalas
[Adam Harvey]: https://github.com/LawnGnome
[Alejandra González]: https://github.com/blyxyas
[Alice Ryhl]: https://github.com/Darksonn
[Alona Enraght-Moony]: https://github.com/adotinthevoid
[Amanda Stjerna]: https://github.com/amandasystems
[Amanieu d'Antras]: https://github.com/Amanieu
[Arlo Siemsen]: https://github.com/arlosi
[Benno Lossin]: https://github.com/BennoLossin
[Boxy]: https://github.com/BoxyUwU
[Complete]: https://img.shields.io/badge/Complete-green
[Connor Horman]: https://github.com/chorman0773
[Crystal Durham]: https://github.com/CAD97
[David Tolnay]: https://github.com/dtolnay
[David Wood]: https://github.com/davidtwco
[Deadbeef]: https://github.com/fee1-dead
[Ding Xiang Fei]: https://github.com/dingxiangfei2009
[Dirkjan Ochtman]: https://github.com/djc
[Ed Page]: https://github.com/epage
[Eric Holk]: https://github.com/eholk
[Eric Huss]: https://github.com/ehuss
[Folkert de Vries]: https://github.com/folkertdev
[Frank King]: https://github.com/frank-king
[Guillaume Gomez]: https://github.com/GuillaumeGomez
[Help wanted]: https://img.shields.io/badge/Help%20wanted-yellow
[Ian McCormack]: https://github.com/icmccorm
[Jack Huey]: https://github.com/jackh726
[Jack Wrenn]: https://github.com/jswrenn
[Jakob Koschel]: https://github.com/jakos-sec
[Jakub Beránek]: https://github.com/Kobzol
[Jana Dönszelmann]: https://github.com/jdonszelmann
[Jane Lusby]: https://github.com/yaahc
[Josh Triplett]: https://github.com/joshtriplett
[Mads Marquart]: https://github.com/madsmtm
[Manuel Drehwald]: https://github.com/ZuseZ4
[Mario Carneiro]: https://github.com/digama0
[Mark Rousskov]: https://github.com/Mark-Simulacrum
[Matthew Jasper]: https://github.com/matthewjasper
[Niko Matsakis]: https://github.com/nikomatsakis
[Not funded]: https://img.shields.io/badge/Not%20yet%20funded-red
[Oliver Scherer]: https://github.com/oli-obk
[Pete LeVasseur]: https://github.com/PLeVasseur
[Predrag Gruevski]: https://github.com/obi1kenobi
[Ralf Jung]: https://github.com/RalfJung
[Ramon de C Valle]: https://github.com/rcvalle
[Ross Sullivan]: https://github.com/ranger-ross
[Rémy Rakic]: https://github.com/lqd
[Santiago Pastorino]: https://github.com/spastorino
[Scott McMurray]: https://github.com/scottmcm
[Sparrow Li]: https://github.com/SparrowLii
[TBD]: https://img.shields.io/badge/TBD-red
[TC]: https://github.com/traviscross
[Takayuki Maeda]: https://github.com/TaKO8Ki
[Taylor Cramer]: https://github.com/cramertj
[Team]: https://img.shields.io/badge/Team%20ask-red
[Tomas Sedovic]: https://github.com/tomassedovic
[Tyler Mandry]: https://github.com/tmandry
[Vadim Petrochenkov]: https://github.com/petrochenkov
[Weihang Lo]: https://github.com/weihanglo
[Wesley Wiser]: https://github.com/WesleyWiser
[Yoshua Wuyts]: https://github.com/yoshuawuyts
[Yuki Okushi]: https://github.com/JohnTitor
[all]: https://www.rust-lang.org/governance/teams
[alumni]: https://www.rust-lang.org/governance/teams
[android]: https://www.rust-lang.org/governance/teams
[apple]: https://www.rust-lang.org/governance/teams
[arewewebyet]: https://www.rust-lang.org/governance/teams
[arm]: https://www.rust-lang.org/governance/teams
[arm-maintainers]: https://www.rust-lang.org/governance/teams
[beyond-refs-editors]: https://www.rust-lang.org/governance/teams
[bjorn3]: https://github.com/bjorn3
[book]: https://github.com/rust-lang/book
[bootstrap]: https://github.com/rust-lang/rust
[cargo]: https://github.com/rust-lang/cargo
[clippy]: https://github.com/rust-lang/rust-clippy
[clippy-contributors]: https://github.com/rust-lang/rust-clippy
[cloud-compute]: https://www.rust-lang.org/governance/teams
[codegen-c-maintainers]: https://github.com/rust-lang/rustc_codegen_c
[community]: https://github.com/rust-community/team
[community-events]: https://github.com/rust-community/events-team
[community-localization]: https://github.com/rust-lang/community-localization
[compiler]: http://github.com/rust-lang/compiler-team
[compiler-fcp]: http://github.com/rust-lang/compiler-team
[compiler-ops]: https://www.rust-lang.org/governance/teams
[content]: https://github.com/rust-lang/content-team
[cookbook]: https://github.com/rust-lang-nursery/rust-cookbook/
[council-librarians]: https://www.rust-lang.org/governance/teams
[crate-maintainers]: https://www.rust-lang.org/governance/teams
[crates-io]: https://github.com/rust-lang/crates.io
[crates-io-admins]: https://www.rust-lang.org/governance/teams
[crates-io-infra-admins]: https://www.rust-lang.org/governance/teams
[crates-io-on-call]: https://www.rust-lang.org/governance/teams
[devtools]: https://github.com/rust-dev-tools/dev-tools-team
[dianne]: https://github.com/dianne
[docker]: https://github.com/rust-lang/docker-rust/
[docs-rs]: https://github.com/rust-lang/docs.rs
[docs-rs-reviewers]: https://github.com/rust-lang/docs.rs
[edition]: http://github.com/rust-lang/edition-team
[emacs]: https://www.rust-lang.org/governance/teams
[emscripten]: https://www.rust-lang.org/governance/teams
[expect-test]: https://www.rust-lang.org/governance/teams
[faq]: #frequently-asked-questions
[fls]: http://github.com/rust-lang/fls-team
[fls-contributors]: https://www.rust-lang.org/governance/teams
[foundation-board-project-directors]: https://www.rust-lang.org/governance/teams
[foundation-email-redirects]: https://www.rust-lang.org/governance/teams
[foundation-staff]: https://www.rust-lang.org/governance/teams
[fuchsia]: https://www.rust-lang.org/governance/teams
[goal-owners]: https://www.rust-lang.org/governance/teams
[goals]: https://github.com/rust-lang/rust-project-goals
[gpu-target]: https://www.rust-lang.org/governance/teams
[gsoc-contributors]: https://www.rust-lang.org/governance/teams
[guide-level-explanation]: #guide-level-explanation
[hiring]: https://www.rust-lang.org/governance/teams
[infra]: https://github.com/rust-lang/infra-team
[infra-admins]: https://www.rust-lang.org/governance/teams
[infra-bors]: https://github.com/rust-lang/bors
[infra-bors-admins]: https://www.rust-lang.org/governance/teams
[inside-rust-reviewers]: https://www.rust-lang.org/governance/teams
[internal-sites]: https://www.rust-lang.org/governance/teams
[lang]: http://github.com/rust-lang/lang-team
[lang-advisors]: https://www.rust-lang.org/governance/teams
[lang-docs]: https://www.rust-lang.org/governance/teams
[lang-ops]: https://www.rust-lang.org/governance/teams
[launching-pad]: https://www.rust-lang.org/governance/teams
[lcnr]: https://github.com/lcnr
[leadership-council]: https://github.com/rust-lang/leadership-council
[leads]: https://www.rust-lang.org/governance/teams
[libs]: https://github.com/rust-lang/libs-team
[libs-api]: https://www.rust-lang.org/governance/teams
[libs-contributors]: https://www.rust-lang.org/governance/teams
[loongarch]: https://www.rust-lang.org/governance/teams
[mentors]: https://www.rust-lang.org/governance/teams
[mentorship]: https://www.rust-lang.org/governance/teams
[miri]: https://github.com/rust-lang/miri
[mods]: https://github.com/rust-lang/moderation-team
[mods-discourse]: https://www.rust-lang.org/governance/teams
[mods-venue]: https://www.rust-lang.org/governance/teams
[motivation]: #motivation
[opsem]: https://github.com/rust-lang/opsem-team
[ospp]: https://www.rust-lang.org/governance/teams
[ospp-contributors]: https://www.rust-lang.org/governance/teams
[program]: https://www.rust-lang.org/governance/teams
[project-async-crashdump-debugging]: https://github.com/rust-lang/async-crashdump-debugging-initiative
[project-const-generics]: https://github.com/rust-lang/project-const-generics
[project-const-traits]: https://github.com/rust-lang/project-const-traits
[project-dyn-upcasting]: https://github.com/rust-lang/dyn-upcasting-coercion-initiative
[project-exploit-mitigations]: https://github.com/rust-lang/project-exploit-mitigations
[project-goal-reference-expansion]: https://www.rust-lang.org/governance/teams
[project-group-leads]: https://www.rust-lang.org/governance/teams
[project-impl-trait]: https://github.com/rust-lang/impl-trait-initiative
[project-keyword-generics]: https://github.com/rust-lang/keyword-generics-initiative
[project-negative-impls]: https://github.com/rust-lang/negative-impls-initiative
[project-portable-simd]: https://www.rust-lang.org/governance/teams
[project-stable-mir]: https://github.com/rust-lang/project-stable-mir
[project-trait-system-refactor]: https://github.com/rust-lang/types-team
[project-vision-doc-2025]: https://github.com/rust-lang/vision-doc-2025
[rationale-and-alternatives]: #frequently-asked-questions
[reference-level-explanation]: #reference-level-explanation
[regex]: https://github.com/rust-lang/regex
[release]: https://github.com/rust-lang/release-team
[release-publishers]: https://github.com/rust-lang/release-team
[relnotes-interest-group]: https://www.rust-lang.org/governance/teams
[rfmf-design-committee]: https://www.rust-lang.org/governance/teams
[risc-v]: https://www.rust-lang.org/governance/teams
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
[rust-analyzer-contributors]: https://github.com/rust-lang/rust-analyzer
[rust-by-example]: https://github.com/rust-lang/rust-by-example
[rust-for-linux]: https://www.rust-lang.org/governance/teams
[rust-timer]: https://www.rust-lang.org/governance/teams
[rustc-dev-guide]: https://forge.rust-lang.org/compiler/working-areas.html
[rustconf-emails]: https://www.rust-lang.org/governance/teams
[rustdoc]: https://github.com/rust-lang/rust
[rustdoc-frontend]: https://www.rust-lang.org/governance/teams
[rustfmt]: https://github.com/rust-lang/rustfmt
[rustfmt-contributors]: https://github.com/rust-lang/rustfmt
[rustlings]: https://github.com/rust-lang/rustlings/
[rustup]: https://github.com/rust-lang/rustup
[security-response]: https://github.com/rust-lang/wg-security-response
[social-media]: https://www.rust-lang.org/governance/teams
[spec]: https://github.com/rust-lang/spec
[spec-contributors]: https://github.com/rust-lang/spec
[style]: https://github.com/rust-lang/style-team
[summary]: #summary
[survey]: https://github.com/rust-lang/surveys
[team-repo-admins]: https://www.rust-lang.org/governance/teams
[teor]: https://github.com/teor2345
[testing-devex]: https://www.rust-lang.org/governance/teams
[tiif]: https://github.com/tiif
[triage]: https://www.rust-lang.org/governance/teams
[triagebot]: https://github.com/rust-lang/triagebot
[twir]: https://github.com/rust-lang/this-week-in-rust
[twir-reviewers]: https://github.com/rust-lang/this-week-in-rust
[types]: https://github.com/rust-lang/types-team
[types-fcp]: https://github.com/rust-lang/types-team
[vim]: https://www.rust-lang.org/governance/teams
[waffle]: https://github.com/WaffleLapkin
[walterhpearce]: https://github.com/walterhpearce
[wasi]: https://www.rust-lang.org/governance/teams
[wasm]: https://www.rust-lang.org/governance/teams
[web-presence]: https://www.rust-lang.org/governance/teams
[website]: https://github.com/rust-lang/www.rust-lang.org/
[wg-allocators]: https://github.com/rust-lang/wg-allocators
[wg-async]: https://github.com/rust-lang/wg-async
[wg-bindgen]: https://github.com/rust-lang/rust-bindgen
[wg-cli]: https://www.rust-lang.org/governance/teams
[wg-compiler-performance]: https://github.com/rust-lang/rustc-perf
[wg-const-eval]: https://github.com/rust-lang/const-eval
[wg-diagnostics]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-embedded]: https://github.com/rust-embedded/wg
[wg-embedded-arm]: https://www.rust-lang.org/governance/teams
[wg-embedded-core]: https://www.rust-lang.org/governance/teams
[wg-embedded-hal]: https://www.rust-lang.org/governance/teams
[wg-embedded-infra]: https://www.rust-lang.org/governance/teams
[wg-embedded-libs]: https://www.rust-lang.org/governance/teams
[wg-embedded-linux]: https://www.rust-lang.org/governance/teams
[wg-embedded-msp430]: https://www.rust-lang.org/governance/teams
[wg-embedded-resources]: https://www.rust-lang.org/governance/teams
[wg-embedded-riscv]: https://www.rust-lang.org/governance/teams
[wg-embedded-tools]: https://www.rust-lang.org/governance/teams
[wg-embedded-triage]: https://www.rust-lang.org/governance/teams
[wg-ffi-unwind]: https://github.com/rust-lang/project-ffi-unwind
[wg-gamedev]: https://github.com/rust-gamedev
[wg-gcc-backend]: https://github.com/rust-lang/rustc_codegen_gcc
[wg-inline-asm]: https://github.com/rust-lang/project-inline-asm
[wg-leads]: https://www.rust-lang.org/governance/teams
[wg-linker]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-llvm]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-macros]: https://github.com/rust-lang/wg-macros
[wg-mir-opt]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-parallel-rustc]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-polonius]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-safe-transmute]: https://github.com/rust-lang/project-safe-transmute
[wg-secure-code]: https://github.com/rust-secure-code/wg
[windows]: https://www.rust-lang.org/governance/teams
