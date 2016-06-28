- Feature Name: relative_links
- Start Date: 2016-06-27
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow `::/` as a prefix in documentation links to indicate a module-relative link.

# Motivation
[motivation]: #motivation

Today, when rustdoc comments have links to other sections of rustdoc, the links are standard markdown links translated as-is into HTML hrefs. This has some problems. Doc authors must understand what filesystem level their item will be emitted at, what level the destination item is emitted at, and correctly form the necessary amount of `../` to correct for differences between the two. This is inconvenient and error-prone.

Additionally, some doc strings are processed and output in two different places, and sometimes at two different levels, making it impossible to form a correct link. For example, `libcollections/str.rs` is processed into both `std/collections/btree_set/struct.BTreeSet.html` and `std/collections/struct.BTreeSet.html`, and contains broken links due to attempts to link into `std`.

This change simplifies link formation for doc authors. Instead of this:

``[`Ord`]: ../../std/cmp/trait.Ord.html``

they can now write:

``[`Ord`]: ::/std/cmp/trait.Ord.html``

That will be expanded to `../../std/cmp/trait.Ord.html` and `../../../std/cmp/trait.Ord.html` depending on which is appropriate.

Document authors aren't always human. For example, I am attempting to write an automatic linkifier for rust docs. This link syntax makes tooling easier to write since it allows tools to ignore the module in which an item is located.

# Detailed design
[design]: #detailed-design

The markdown processor processes links like normal, except that an interceptor looks for the literal string `::/` at the start of a link, and replaces that literal with enough `../` to form a path to the module's documentation root.

Everything after the `::/` remains intact and appears raw in the resulting HTML. It's legal and even advisable to link to supplemental documentation-- for example linking to the rust book from libstd-- using this syntax. It's not just for rust items.

The hoedown parser we use supports link processing extensions. It also handles the leading colon correctly.

The `::` part of the spelling was chosen since `::` already means module root. The `/` part of the spelling was chosen to allow for future enhancements (not proposed here) that might allow for more of the rust item name. e.g. `::std::cmp::Ord` as a link.

# Drawbacks
[drawbacks]: #drawbacks

This is not the best possible solution. The best solution to this problem would be to encode the semantic meaning of the link directly at the link location, and leaving HTML href creation entirely to the processor. For example:

> The :rust{std::cmp::Ord} trait is the grooviest trait there is!

as opposed to what this RFC proposes:

> The [\`Ord`] trait is the grooviest trait there is!
> 
> [\`Ord`]: ::/std/cmp/trait.Ord.html

The benefit of the first syntax is that it's a bit more concise, would engender more standardization of links, and would help with doc tooling such as refactoring following.

This RFC doesn't propose the first syntax because it would best be written as a CommonMark extension and a) we don't currently use a CommonMark processor, and b) CommonMark has [not yet][commonmark-plugin] standardized extensions. This proposal is a small short term fix for immediate pain, and does not preclude the semantic link scheme once that becomes viable. `::/` style links may still be desired even when we have semantic links in order to support links to things that aren't rust items, but are contained in the doc tree.

Another drawback is that this change will make it slightly harder to switch to a new markdown processor, since the new processor will also have to support this behavior.

# Alternatives
[alternatives]: #alternatives

### Spelling

This could just be spelled `/`, which is what a naive user might do anyway. The behavior could be surprising to a naive user, but I don't see the harm in that. I can't think of a case where you'd really want to force a true root-relative link with the classic meaning of `/`.

This could be spelled `rust:/`, which is attractive since we could consider the `rust:` part to be a URI scheme. In practice, `rust:/` is a bit longer, and because it looks less strange, it's easier to miss that it's special. Also, it "uses up" the `rust:` scheme. We might want to do something richer with it in the future such as use it for semantic linking.

This could be spelled `/::/` if there's worry about colon processing in future markdown processors.

### Semantic Links

Maybe we don't need to wait for CommonMark extensions and a new markdown processor to support semantic links. Maybe we can use standard links today with this scheme, like:

> The [\`Ord\`]\(::std::cmp::Ord) trait is the grooviest trait there is!

Since this is more complicated, not precluded by my proposal, and doesn't help with non-item links, I think it's best left to the future.

### Cross-Crate Links

We could add a slight bit of semantic meaning by allowing a crate name to be provided, like `::crate_name/`. This doesn't buy much since rustdoc places docs for dependencies in sibling trees to the main crate docs, so `::/crate_name/` works well. Also, adding that syntax would require either error handling for cases when no dependencies are documented, or some way to form absolute links. This form would most benefit linking to libstd since those docs are not generated for each crate consumer.

# Unresolved questions
[unresolved]: #unresolved-questions

Unresolved questions are largely mentioned inline in Alternatives and Drawbacks.

Should there be error handling to forbid constructions like `::foo/` that we won't support initially, but may support in the future? Given the current proposal that constuction would be rendered literally.

[commonmark-plugin]: https://talk.commonmark.org/t/generic-directives-plugins-syntax
