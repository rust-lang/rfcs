- Feature Name: `rustdoc_scrape_examples`
- Start Date: 2021-05-09
- RFC PR: [rust-lang/rfcs#3123](https://github.com/rust-lang/rfcs/pull/3123)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes an extension to Rustdoc that automatically scrapes code examples from the project's `examples/` directory. 

Check out a live demo here: https://willcrichton.net/example-analyzer/warp/trait.Filter.html#method.and

# Motivation
[motivation]: #motivation

Code examples are an important tool for programmers to understand how a library works. Examples are **concrete** and **contextual**: they reference actual values rather than abstract parameters, and they show how to use a function in the context of the code around it. 

As a parable of the value of examples, I recently did a small user study where I observed two Rust programmers learning to use the [warp](https://github.com/seanmonstar/warp) library for a basic task. Warp is designed around a generic [`Filter`](https://docs.rs/warp/0.3.1/warp/trait.Filter.html) abstraction. Both participants found the documentation for `Filter` methods to be both imposing and too generic to be useful. For instance, [`Filter::and`](https://docs.rs/warp/0.3.1/warp/trait.Filter.html#method.and):

<kbd>
<img width="600" alt="Screen Shot 2021-05-09 at 7 49 35 PM" src="https://user-images.githubusercontent.com/663326/117592915-fe07b880-b0ff-11eb-97e9-43197fbcb2a7.png">
</kbd>
<br /><br />


The repo authors also included a code example. But neither participant could understand the example because it lacked context.

<kbd>
<img width="450" alt="Screen Shot 2021-05-09 at 7 56 03 PM" src="https://user-images.githubusercontent.com/663326/117593130-a87fdb80-b100-11eb-9a11-ef57ec0d4872.png">
</kbd>
<br /><br />


The participant who was less familiar with Rust struggled to read the documentation and failed to accomplish the task. By contrast, the participant who was more familiar with Rust knew to look in the `examples/` directory, where they found a wealth of examples for each function that complemented the documentation. For instance, [`rejection.rs`](https://github.com/seanmonstar/warp/blob/bf8bfc4134035dbff882f9b26cb9d1aa57f2c338/examples/rejections.rs) shows the usage of `and` in combination with `map`:

```rust
let math = warp::path!("math" / u16);
let div_with_header = math
    .and(warp::get())
    .and(div_by())
    .map(|num: u16, denom: NonZeroU16| {
        warp::reply::json(&Math {
            op: format!("{} / {}", num, denom),
            output: num / denom.get(),
        })
    });
```

The goal of this RFC is to bridge the gap between automatically generated documentation and code examples by helping users find relevant examples within Rustdoc.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `scrape-examples` feature of Rustdoc finds examples of code where a particular function is called. For example, if we are documenting [`Filter::and`](https://willcrichton.net/example-analyzer/warp/trait.Filter.html#method.and), and a file [`examples/returning.rs`](https://github.com/seanmonstar/warp/tree/bf8bfc4134035dbff882f9b26cb9d1aa57f2c338/examples/returning.rs) contains a call to `and`, then the corresponding Rustdoc documentation looks like this:

<kbd>
<img width="982" alt="Screen Shot 2021-05-11 at 11 31 12 AM" src="https://user-images.githubusercontent.com/663326/117852001-81cebb80-b24c-11eb-88ef-8532a5f012c4.png">
</kbd>
<br /><br />

After the user-provided documentation in the doc-comment, `scrape-examples` inserts a code example (if one exists). The code example shows a window into the source file with the function call highlighted in yellow. The icons in the top-right of the code viewer allow the user to expand the code sample to the full file, or to navigate through other calls in the same file. The link above the example goes to the example in the crate's repository.

Additionally, the user can click "More examples" to see every example from the `examples/` directory, like this:

<kbd>
  <img width="956" alt="Screen Shot 2021-05-11 at 11 31 36 AM" src="https://user-images.githubusercontent.com/663326/117852026-8abf8d00-b24c-11eb-819d-51627798e005.png">

</kbd>
<br /><br />

To use the `scrape-examples` feature, simply add the `--scrape-examples` flag like so:

```
cargo doc --scrape-examples
```


# Reference-level explanation

I have implemented a prototype of the `scrape-examples` feature as modifications to rustdoc and cargo. You can check out the diffs: 
* rustdoc: https://github.com/willcrichton/rust/compare/master...willcrichton:example-analyzer?expand=1
* cargo: https://github.com/willcrichton/cargo/compare/master...willcrichton:example-analyzer?expand=1

The feature uses the following high-level flow, with some added technical details as necessary.

1. The user gives `--scrape-examples` as an argument to `cargo doc`.
2. Cargo runs the equivalent of `cargo build --examples` ([source](https://github.com/willcrichton/cargo/blob/fd25a0301314a9eba6beb5239891fc5902a9a9a9/src/cargo/ops/cargo_compile.rs#L618-L631)).
    *  Specifically, for each unit being documented, it copies the Config and CliFeatures from the input CompileOpts. Then it sets the CompileFilter to only match examples.    
4. Cargo generates build flags for each example. ([source](https://github.com/willcrichton/cargo/blob/fd25a0301314a9eba6beb5239891fc5902a9a9a9/src/cargo/ops/cargo_compile.rs#L633-L646)).
    * This is implemented by repurposing the `Doctest` target, which also is used to generate build flags to pass to rustdoc.
6. Cargo identifies a remote repository URL for linking to the examples ([source](https://github.com/willcrichton/cargo/blob/fd25a0301314a9eba6beb5239891fc5902a9a9a9/src/cargo/ops/cargo_compile.rs#L594-L608)).
    * Currently this is done by retrieving `package.repository` from the manifest and casing on the domain name. If examples were packaged with rustdoc like other source files, then this could instead link to the generated `src` directory.
7. Cargo invokes rustdoc with added flags: `--repository-url https://github.com/... --scrape-examples "rustc examples/foo.rs --extern ..."`
9. Rustdoc iterates through each example and uses a visitor to identify spans of calls to functions in the crate being documented ([source](https://github.com/willcrichton/rust/blob/2653c671a4ae89070fdf00f9e149486146e7fc18/src/librustdoc/scrape_examples.rs)).
    * This means that rustc is invoked multiple times within a single process before the core of rustdoc is actually executed. Care will be needed to avoid issues with global state like the string interner.
11. Rustdoc adds the scraped examples to the documentation for each function ([source](https://github.com/willcrichton/rust/blob/2653c671a4ae89070fdf00f9e149486146e7fc18/src/librustdoc/html/render/mod.rs#L2394-L2471)).
12. Rustdoc's Javascript adds interactivity to the examples when loaded ([source](https://github.com/willcrichton/rust/blob/2653c671a4ae89070fdf00f9e149486146e7fc18/src/librustdoc/html/static/main.js#L1415-L1599)).
    * Most of the logic here is to extend the code viewer with additional features like toggling between snippet / full file, navigating between call sites, and highlighting code in-situ.

The primary use case for this will be on docs.rs. My expectation is that docs.rs would use the `--scrape-examples` flag, and all docs hosted there would have the scraped examples.

# Drawbacks
[drawbacks]: #drawbacks

1. I think the biggest drawback of this feature is that it adds further complexity to the Rustdoc interface. Rustdoc already includes a lot of information, and a concern is that this feature would overload users, especially Rust novices.
2. This feature requires pre-loading a significant amount of information into the HTML pages. If we want to keep the "view whole file" feature, then the entire source code of every referenced example would be embedded into every page. This will increase the size of the generated files and hence increase page load times.
3. This feature requires adding more functionality to both Cargo and Rustdoc, increasing the complexity of both tools.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* At the highest-level, this tool could be built separately from Rustdoc as an independent database mapping functions to examples. I believe it's preferable to have the function -> example connections integrated into Rustdoc so people need as few tools as possible to understand Rust libraries. Moreover, Rustdoc-generated pages are the main results that appear in Google when searching for help with Rust libraries, so it's the easiest for programmers to find.
* At the lower-level, this feature could be implemented in alternative interfaces. For instance, examples could be a separate part of the Rustdoc interface. I'm not sure what this interface would look like -- having the examples be inline was the only sensible interface I could imagine.

See "Unresolved questions" for more discussion of the design space.

# Prior art
[prior-art]: #prior-art

I have never seen a documentation generator with this exact feature before. There has been some HCI research like [Jadeite](https://dl.acm.org/doi/pdf/10.1145/1520340.1520678) and [Apatite](https://dl.acm.org/doi/pdf/10.1145/1753326.1753525) that use code examples to augment generated documentation, e.g. by sorting methods in order of usage. Other research prototypes have clustered code examples to show broad usage patterns, e.g. [Examplore](https://dl.acm.org/doi/pdf/10.1145/3173574.3174154).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. **UI design:** What is the best UI to show the examples inline? My prototype represents my best effort at a draft, but I'm open to suggestions. For example:
    * Is showing 1 example by default the best idea? Or should scraped examples be hidden by default? 
    * Is the ability to see the full file context worth the increase in page size?
    * How should the examples be ordered? Is there a way to determine the "best" examples to show first?
2. **Tooling integration:** Are there better ways to accomplish the tooling sub-tasks? Specifically:
    * Is there a robust way of generating links to examples based on the Cargo.toml `package.repository` field, especially that generalizes across choice of VCS? Is there a way to reliably get the current commit so as to generate stable links?
    * Is invoking rustc on each example within rustdoc the best way to analyze the examples for call sites? In my [original prototype](https://github.com/willcrichton/example-analyzer), I wrote a standalone tool that output JSON which was then read in by Rustdoc. One benefit of this approach is that Rustdoc could then integrate with any tool that analyzes call sites. But the downside is requiring yet another tool to be in-tree.
    * What is the best way to handle Cargo workspaces? For example, some workspaces like [wasmtime](https://github.com/bytecodealliance/wasmtime) have a single examples directory at the root with many crates in a `crates/` subfolder. However, under my current strategy for finding examples, they would only be scraped during documentation of the root crate, not the other crates in the workspace.

# Future possibilities
[future-possibilities]: #future-possibilities

To my mind, the main future extensions of this feature are:
1. **More examples:** examples can be scraped from the codebase itself (e.g. this would be very useful for developers on large code bases like rustc ), or scraped from the ecosystem at large.
2. **Ordering examples:** with more examples comes the question of how to present them all to the user. If there are too many examples, say >10, there should be a way to maximize the diversity of the examples (or something like that).
