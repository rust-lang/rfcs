- Feature Name: add_amdgpu_and_ptx_target
- Start Date: 2016-06-04
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This would be the addition of AMDGPU and PTX targets for Rust, allowing Rust to run directly on the GPU without weird tricks and workarounds. There was an attempt at adding GPU support for rust a [few years ago](http://blog.theincredibleholk.org/blog/2012/12/05/compiling-rust-for-gpus/), but there hasn't really been anything since. [This](http://llvm.org/docs/NVPTXUsage.html) is the page for LLVM PTX backend and [this](http://llvm.org/docs/AMDGPUUsage.html) is the page for the LLVM AMDGPU backend.

# Motivation
[motivation]: #motivation

LLVM currently supports a number of targets that currently are not supported by Rust itself. Two of these are AMDGPU and PTX targets, NVIDIA's and AMD's respective GPU assembly languages. Working these in would allow people to target the GPU directly and, since they are both already supported by LLVM, this seems to be the cleanest and most realistic option for adding GPU support to Rust. Once these targets are added, then people can start building libraries around GPU usage. There are already bindings for [OpenCL](https://github.com/luqmana/rust-opencl), [ArrayFire](https://github.com/arrayfire/arrayfire-rust), and [Vulcan](https://github.com/tomaka/vulkano), so there seems to be interest in running Rust in and around the GPU. This would be a relatively simple change that could allow Rust to find many more use cases.

# Detailed design
[design]: #detailed-design

To be clear, the change I am proposing is not to make the PTX or AMDGPU targets perfect or to add all of the special LLVM calls that they include, but rather to put a rudimentary and potentially unstable PTX or AMDGPU target out for evaluation and potential improvement. The primary change involved here is adding to the list of targets in [`src/librustc_back/target`](https://github.com/rust-lang/rust/tree/master/src/librustc_back/target) and adding the tests to support it. The largest part of this change will be extensive testing to ensure that the PTX and AMDGPU targets operate as expected. Most instructions are currently supported, but it will be good to test and see if anything is off so that can either be fixed or disabled for either of the targets.

Eventually there could be a more established pathway for building a single source binary for running programs on the GPU, but that also is somewhat outside the scope of this RFC.

This pathway would adhere to the current standard and would produce a binary for either of these platforms by using the compiler `target` flag.

The target definition would be fairly close to the following for PTX:

    Target {
          llvm_target: "ptx".to_string(),
          target_endian: "little".to_string(),
          target_pointer_width: "64".to_string(),
          data_layout: "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v32:32:32-v64:64:64-v128:128:128-n16:32:64".to_string(),
          arch: "NVPTX".to_string(),
          target_os: "".to_string(),
          target_env: "".to_string(),
          target_vendor: "unknown".to_string()
    }

And similar with AMDGPU.

# Drawbacks
[drawbacks]: #drawbacks

I originally did not want to go this route if it could be avoided, because in most cases in order to get use out of a PTX or AMDGPU binary, you need to have an external library (such as OpenCL). Not only that, but the LLVM targets depend largely on NVIDIA and AMD updating the LLVM targets. This is a somewhat ugly situation, because a language should not rely on third party libraries for functionality. I still think that this is a genuine concern, but I think that the benefits outweigh the risks.

# Alternatives
[alternatives]: #alternatives

I tried to use the Khronos Group's LLVM-SPIRV cross compiler, but they built that so deeply into the OpenCL C infrastructure that it is virtually impossible to work out. It is also possible to compile Rust to LLVM and compile the LLVM into PTX or AMDGPU, but this is an added step involving additional software.

# Unresolved questions
[unresolved]: #unresolved-questions

This change seems fairly small and incremental. I think that the more significant questions pop up when trying to add further GPU support, which I think is could be necessary, but is outside the scope of this RFC.
