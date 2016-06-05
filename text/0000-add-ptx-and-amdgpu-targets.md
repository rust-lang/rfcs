- Feature Name: add_amdgpu_and_ptx_target
- Start Date: 2016-06-04
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This would be the addition of AMDGPU and PTX targets for Rust, allowing Rust to run directly on the GPU without weird tricks and workarounds. There was an attempt at adding GPU support for rust a [few years ago](http://blog.theincredibleholk.org/blog/2012/12/05/compiling-rust-for-gpus/), but there hasn't really been anything since.

# Motivation
[motivation]: #motivation

LLVM currently supports a number of targets that currently are not supported by Rust itself. Two of these are AMDGPU and PTX targets. Working these in would allow people to target the GPU directly and since they are both already supported by LLVM, this seems to be the cleanest and most realistic option for adding GPU support to Rust. Once Rust has these targets then people can build libraries around GPU usage. There are already bindings for [OpenCL](https://github.com/luqmana/rust-opencl), [ArrayFire](https://github.com/arrayfire/arrayfire-rust), and [Vulcan](https://github.com/tomaka/vulkano), so there seems to be interest. This would be a relatively simple change that could allow Rust to find many more use cases.

# Detailed design
[design]: #detailed-design

This change is fairly simple, so this section won't be terribly long. The primary change involved here is adding to the list of targets in `src/librustc_back/target` and adding the tests to support it. The largest part of this change will be extensive testing to ensure that the PTX and AMDGPU targets operate as expected. Most instructions are currently supported, but it will be good to test and see if anything is off so that can either be fixed or disabled for either of the targets.

# Drawbacks
[drawbacks]: #drawbacks

I originally did not want to go this route if it could be avoided, because in most cases in order to get use out of a PTX or AMDGPU binary, you need to have an external library (such as OpenCL). This is a somewhat ugly situation, because a language should not rely on third party libraries for functionality. I still think that this is a genuine concern, but I think that the benefits outweigh the risks.

# Alternatives
[alternatives]: #alternatives

I tried to use the Khronos Group's LLVM-SPIRV cross compiler, but they built that so deeply into the C infrastructure that it is virtually impossible to work out. It is also possible to compile Rust to LLVM and compile the LLVM into PTX or AMDGPU.

# Unresolved questions
[unresolved]: #unresolved-questions

This change seems fairly small and incremental. I think that the more significant questions pop up when trying to add further GPU support, which I think is unnecessary.
