- Feature Name: `cargo_embed_dependency_versions`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: None

# Summary
[summary]: #summary

Embed information equivalent to the contents of Cargo.lock into compiled binaries so it could be programmatically recovered later.

# Motivation
[motivation]: #motivation

Rust is very promising for security-critical applications due to its safety guarantees, but there currently are gaps in the ecosystem that prevent it. One of them is the lack of any infrastructure for security updates.

Linux distributions alert you if you're running a vulnerable software version and you can opt in to automatic security updates. Cargo not only has no automatic update infrastructure, it doesn't even know which libraries or library versions went into compiling a certain binary, so there's no way to check if your system is vulnerable or not.

The primary use case for this information is cross-referencing versions of the dependencies against [RustSec advisory database](https://github.com/RustSec/advisory-db) and/or [Common Vulnerabilities and Exposures](https://en.wikipedia.org/wiki/Common_Vulnerabilities_and_Exposures). This also enables use cases such as ensuring a fix in a depencency has been propagated across the entirety of your fleet or preventing binaries with unvetted dependencies from accidentally reaching a production environment - all with zero bookkeeping.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Every time an executable is compiled with Cargo, the contents of Cargo.lock are embedded in the generated binary. It can be recovered using existing tools like `readelf` or Rust-specific tooling, and then inspected manually or processed in an automated way just like the regular `Cargo.lock` file.

WASM, asm.js and embedded platforms excempt from this mechanism since they have very strict code size requirements. For those platforms we encourage you to use tooling that record the hash of every executable in a database and associates the hash with its Cargo.lock, compiler and LLVM version used for the build.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The version information is encoded in an additional arbitrary section of the executable (PE, ELF and Mach-O all allow arbitrary sections) by Cargo. Section name is subject to bikeshedding.

For each crate in the dependency tree, including the root crate, the recorded version information contains the name, version, origin URL and checksum (equivalent to the current contents of `Cargo.lock` file). The exact format is TBD - see [unresolved questions](#unresolved-questions).

A prototype implementation for Linux in `bash` looks like this:

```shell
# Insert Cargo.lock into a new '.dep-list' section
objcopy --add-section .dep-list=Cargo.lock --set-section-flags .dep-list=noload,readonly mybinary mybinary.withdeps

# Extract Cargo.lock
objcopy -O binary --set-section-flags .dep-list=alloc --only-section=.dep-list mybinary.withdeps Cargo.lock.extracted
```

# Drawbacks
[drawbacks]: #drawbacks

- Slightly increases the size of the generated binaries. However, the increase is below 1%. A "Hello World" on x86 Linux compiles into a ~1Mb file in the best case (recent Rust without jemalloc, LTO enabled). Its Cargo.lock even with a couple of dependencies is less than 1Kb, that's under 1/1000 of the size of the binary. Since Cargo.lock grows linearly with the number of dependencies, it will keep being negligible.
- Adds more platform-specific code to the build process, which needs to be maintained.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rationale:

- This way version information is *impossible* to misplace. As long as you have the binary, you can recover the info about dependency versions. The importance of this cannot be overstated. This allows auditing e.g. a Docker container that you did not build yourself, or a server that somebody's set up a year ago and left no audit trail.
- A malicious actor could lie about the version information. However, doing so requires modifying the binary - and if a malicious actor can do _that,_ you are pwned anyway. So this does not create any additional attack vectors other than exploiting the tool that's recovering the version information, which can be easily sandboxed.
- Any binary authentication that might be deployed automatically applies to the version information. There is no need to separately authenticate it.
- Tooling for extracting information from binaries (such as ELF sections) is already readily available. Tooling for parsing `Cargo.lock` also exists.
- This enables third parties such as cloud providers to scan your binaries for you. Google Cloud [already provides such a service](https://cloud.google.com/container-registry/docs/get-image-vulnerabilities), Amazon has [an open-source project you can deploy](https://aws.amazon.com/blogs/publicsector/detect-vulnerabilities-in-the-docker-images-in-your-applications/) while Azure [integrates several partner solutions](https://docs.microsoft.com/en-us/azure/security-center/security-center-vulnerability-assessment-recommendations).

Alternatives:

- Do nothing.
  - Identifying vulnerable binaries will remain impossible. We will see increasing number of known vulnerabilities unpatched in production.
- Track version information separately from the binaries, recording it when running `cargo install` and surfacing it through some other Cargo subcommand. When installing not though `cargo install`, rely on Linux package managers to track version information.
  - Identifying vulnerable binaries will remain impossible on all other platforms, as well as on Linux for code compiled with `cargo build`.
  - Verification by third parties will remain impossible.
- Record version information in a `&'static str` in the binary instead if ELF sections, with start/stop markers to allow black-box extraction from the outside.
  - This has been [prototyped](https://github.com/Shnatsel/rust-audit). It has the upside of allowing the binary itself to introspect its version info, but appears to be harder to implement and maintain.
- Provide a Cargo wrapper or plugin to implement this, but do not put it in Cargo itself.
  - When people actually need this information (e.g. to check if they're impacted by a vulnerability) it is too late to reach for third-party tooling - the executables have already been built and deployed, and the information is already lost. As such, this mechanism is completely ineffective if it's not enabled by default.

# Prior art
[prior-art]: #prior-art

`rustc` already embeds compiler and LLVM version in the executables built with it. You can see it by running `strings your_executable | grep 'rustc version'`.

The author is not aware of direct prior art in other languages. Since build system and package management system are usually decoupled, most languages did not have the opportunity to implement anything like this.

In microservice environments it is fairly typical to expose an HTTP endpoint returning the application version, see e.g. [example from Go cookbook](https://blog.kowalczyk.info/article/vEja/embedding-build-number-in-go-executable.html). However, this typically does not include versions of the dependencies.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. The format of Cargo.lock is not stabilized and is evolving. Should we encode Cargo.lock as-is and require tooling to track the updates, commit to a stable subset of Cargo.lock or use something else altogether?
1. Should this also apply to shared libraries?
1. Should this information be removed when stripping the binary of debug symbols?
1. Are there any cases where you would _not_ want to allow whoever is running the binary to check it for known vulnerabilities? 

# Future possibilities
[future-possibilities]: #future-possibilities

- Surface dependency information through an HTTP endpoint in a microservice environment. The [proof-of-concept](https://github.com/Shnatsel/rust-audit/issues/2) has a feature request for it. However, this does not require support from Cargo and can be implemented as a crate.
  - Is data embedded in an ELF section accessible to the application itself at runtime?
- Record and surface versions of C libraries statically linked into the Rust executable, e.g. OpenSSL.

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
