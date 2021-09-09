- Feature Name: `rustdoc_scrape_examples`
- Start Date: 2021-05-09
- RFC PR: [rust-lang/rfcs#3123](https://github.com/rust-lang/rfcs/pull/3123)
- Rust Issue: [rust-lang/rust#88791](https://github.com/rust-lang/rust/issues/88791)

# Summary
[summary]: #summary

This RFC proposes an extension to Rustdoc that automatically scrapes code examples from the project's `examples/` directory. 

Check out a live demo here: https://willcrichton.net/example-analyzer/warp/trait.Filter.html#method.and

# Motivation
[motivation]: #motivation

Code examples are an important tool for programmers to understand how a library works. Examples are **concrete** and **contextual**: they reference actual values rather than abstract parameters, and they show how to use a function in the context of the code around it. 

As a parable of the value of examples, I recently did a small user study where I observed two Rust programmers learning to use the [warp](https://github.com/seanmonstar/warp) library for a basic task. Warp is designed around a generic [`Filter`](https://docs.rs/warp/0.3.1/warp/trait.Filter.html) abstraction. Both participants found the documentation for `Filter` methods to be both imposing and too generic to be useful. For instance, [`Filter::and`](https://docs.rs/warp/0.3.1/warp/trait.Filter.html#method.and):


<img width="600" alt="Rustdoc documentation for Filter::and in the warp crate" src="https://user-images.githubusercontent.com/663326/117592915-fe07b880-b0ff-11eb-97e9-43197fbcb2a7.png">

The repo authors also included a code example. But neither participant could understand the example because it lacked context.

<img width="450" alt="Example code for Filter::and" src="https://user-images.githubusercontent.com/663326/117593130-a87fdb80-b100-11eb-9a11-ef57ec0d4872.png">

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



<img width="982" alt="UI for scraped examples shown with Filter::and" src="https://user-images.githubusercontent.com/663326/120575286-a3e3d580-c3d5-11eb-9183-c65aa89f5250.png">

After the user-provided documentation in the doc-comment, `scrape-examples` inserts a code example (if one exists). The code example shows a window into the source file with the function call highlighted in yellow. The icons in the top-right of the code viewer allow the user to expand the code sample to the full file, or to navigate through other calls in the same file. The link above the example goes to the full listing in Rustdoc's generated `src/` directory, similar to other `[src]` links.

Additionally, the user can click "More examples" to see every example from the `examples/` directory, like this:

<img width="956" alt="Additional examples are shown indented under the main example" src="https://user-images.githubusercontent.com/663326/120575318-ae05d400-c3d5-11eb-9a25-990591c1a075.png">

To use the `scrape-examples` feature, simply add the `--scrape-examples` flag like so:

```
cargo doc --scrape-examples
```


# Reference-level explanation

I have implemented a prototype of the `scrape-examples` feature as modifications to rustdoc and cargo. You can check out the draft PRs: 
* rustdoc: https://github.com/rust-lang/rust/pull/85833
* cargo: https://github.com/rust-lang/cargo/pull/9525

The feature uses the following high-level flow, with some added technical details as necessary.

1. The user gives `--scrape-examples` as an argument to `cargo doc`.
2. Cargo runs the equivalent of `cargo rustdoc --examples` ([source](https://github.com/willcrichton/cargo/blob/9c9f86772cbcf49f77119b7471021989e72c9936/src/cargo/ops/cargo_compile.rs#L596-L655)).
    *  Specifically, when constructing the `BuildContext`, Cargo will now recursively invoke `rustdoc` on all files matching the `--examples` filter. 
    *  Each invocation includes a flag `--scrape-examples <output path>` which directs rustdoc to output to a file at the specific location.
3. An instance of rustdoc runs for each example, finding all call-sites and exporting them to a JSON file ([source](https://github.com/willcrichton/rust/blob/20044cd72dc220e787b081ae2139df49c2320471/src/librustdoc/scrape_examples.rs)).
    * A visitor runs over the HIR to find call sites that resolve to a specific linkable function.
    * As a part of this pass, rustdoc also generates source files for the examples, e.g. `target/doc/src/example/foo.rs`. These are then linked to during rendering.
    * The format of the generated JSON is `{function: {file: {locations: [list of spans], other metadata}}}`. See the [`AllCallLocations`](https://github.com/willcrichton/rust/blob/20044cd72dc220e787b081ae2139df49c2320471/src/librustdoc/scrape_examples.rs#L24-L32) type.
4. Rustdoc is then invoked as normal for the package being documented, except with the added flags `--with-examples <path/to/json>` for each generated JSON file. Rustdoc reads the JSON data from disk and stores them in `RenderOptions`.
5. Rustdoc renders the call locations into the HTML ([source](https://github.com/willcrichton/rust/blob/20044cd72dc220e787b081ae2139df49c2320471/src/librustdoc/html/render/mod.rs#L2433-L2508)).
    * This involves reading the source file from disk to embed the example into the page.
6. Rustdoc's Javascript adds interactivity to the examples when loaded ([source](https://github.com/willcrichton/rust/blob/20044cd72dc220e787b081ae2139df49c2320471/src/librustdoc/html/static/main.js#L965-L1135)).
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

The main unresolved questions are about the UI: what is the best UI to show the examples inline? My prototype represents my best effort at a draft, but I'm open to suggestions. For example:

1. Is showing 1 example by default the best idea? Or should scraped examples be hidden by default? 
2. Is the ability to see the full file context worth the increase in page size?
3. How should the examples be ordered? Is there a way to determine the "best" examples to show first?


# Future possibilities
[future-possibilities]: #future-possibilities

To my mind, the main future extensions of this feature are:
1. **More examples:** examples can be scraped from the codebase itself (e.g. this would be very useful for developers on large code bases like rustc ), or scraped from the ecosystem at large.
2. **Ordering examples:** with more examples comes the question of how to present them all to the user. If there are too many examples, say >10, there should be a way to maximize the diversity of the examples (or something like that).
