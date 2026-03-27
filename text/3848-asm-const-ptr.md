- Feature Name: `asm_const_ptr`
- Start Date: 2025-07-09
- RFC PR: [rust-lang/rfcs#3848](https://github.com/rust-lang/rfcs/pull/3848)
- Rust Issue: [rust-lang/rust#128464](https://github.com/rust-lang/rust/issues/128464)

## Summary
[summary]: #summary

The `const` operand to `asm!` and `global_asm!` currently only accepts
integers. Change it to also accept pointer values. The value must be computed
during const evaluation. The operand expands to the name of the symbol that the
pointer references, plus an integer offset when necessary.

## Motivation
[motivation]: #motivation

Right now, the only way to reference a global symbol from inline asm is to use
the `sym` operand type.
```rs
use std::arch::asm;

static MY_GLOBAL: i32 = 10;

fn main() {
    let mut addr: *const i32;
    unsafe {
        asm!(
            "lea {1}(%rip), {0}",
            out(reg) addr,
            sym MY_GLOBAL,
            options(att_syntax)
        );
    }
    assert_eq!(addr, &MY_GLOBAL as *const i32);
}
```
However, the `sym` operand has several limitations:

* It can only be used with a hard-coded path to one specific global.
* It can only reference the global as a whole, not a field of the global.

### Generics and const-evaluation

The `sym` operand lets you use generic parameters:
```rs
#[unsafe(naked)]
extern "C" fn asm_trampoline<T>() {
    naked_asm!(
        "
            tail {}
        ",
        sym trampoline::<T>
    )
}

extern "C" fn trampoline<T>() { ... }
```
And you can compute integers in const evaluation:
```rs
use std::arch::asm;

const fn math() -> i32 {
    1 + 2 + 3
}

fn main() {
    let mut six: i32;
    unsafe {
        asm!(
            "mov ${1}, {0:e}",
            out(reg) six,
            const math(),
            options(att_syntax)
        );
    }
    println!("{}", six);
}
```
However, asm is otherwise incompatible with const eval. Const evaluation is
only usable to compute integer constants; it cannot access symbols. For
example:
```rs
#[unsafe(naked)]
extern "C" fn asm_trampoline<const FAST: bool>() {
    naked_asm!(
        "tail {}",
        sym if FAST { fast_impl } else { slow_impl },
    )
}

extern "C" fn slow_impl() { ... }
extern "C" fn fast_impl() { ... }
```
```text
error: expected a path for argument to `sym`
 --> src/lib.rs:8:13
  |
8 |         sym if FAST { fast_impl } else { slow_impl },
  |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```
And pointers also do not work:
```rs
use std::arch::asm;

trait HasGlobal {
    const PTR: *const Self;
}

static MY_I32: i32 = 42;
impl HasGlobal for i32 {
    const PTR: *const i32 = &MY_I32;
}

fn get_addr<T: HasGlobal>() -> *const T {
    let mut addr: *const T;
    unsafe {
        asm!(
            "lea {1}(%rip), {0}",
            out(reg) addr,
            sym T::PTR,
            options(att_syntax)
        );
    }
    addr
}
```
```text
error: invalid `sym` operand
  --> src/lib.rs:18:13
   |
18 |             sym T::PTR,
   |             ^^^^^^^^^^ is a `*const T`
   |
   = help: `sym` operands must refer to either a function or a static
```
Casting the pointer to `usize` does not help:
```text
error: pointers cannot be cast to integers during const eval
  --> src/lib.rs:18:19
   |
18 |             const T::PTR as usize,
   |                   ^^^^^^^^^^^^^^^
   |
   = note: at compile-time, pointers do not have an integer value
```

The Linux kernel currently works around this limitation by using a macro:
```rs
macro_rules! get_addr {
    ($out:ident, $global:path) => {
        core::arch::asm!(
            "lea {1}(%rip), {0}",
            out(reg) $out,
            sym $global,
            options(att_syntax)
        )
    };
}

static MY_I32: i32 = 42;

fn main() {
    let x: *const i32;
    unsafe { get_addr!(x, MY_I32) };
    println!("{}", unsafe { *x });
}
```
With the macro it is possible to use the `sym` operand to access a global
specified by the caller. However, this has the disadvantage of being a macro
rather than a function call, and you also cannot get around the fact that you
must specify the name of the global directly in the macro invocation.

### Accessing fields

Let's say you want to access the field of a static.
```rs
use std::arch::asm;

#[repr(C)]
struct MyStruct {
    a: i32,
    b: i32,
}

static MY_GLOBAL: MyStruct = MyStruct {
    a: 10,
    b: 42,
};

fn main() {
    let mut addr: *const i32;
    unsafe {
        asm!(
            "lea {1}(%rip), {0}",
            out(reg) addr,
            sym MY_GLOBAL.b,
            options(att_syntax)
        );
    }
    assert_eq!(addr, &MY_GLOBAL.b as *const i32);
}
```
```text
error: expected a path for argument to `sym`
  --> src/main.rs:20:17
   |
20 |             sym MY_GLOBAL.b,
   |                 ^^^^^^^^^^^
```
The only way to fix this is to use `offset_of!`.
```rs
use std::arch::asm;
use std::mem::offset_of;

#[repr(C)]
struct MyStruct {
    a: i32,
    b: i32,
}

static MY_GLOBAL: MyStruct = MyStruct { a: 10, b: 42 };

fn main() {
    let mut addr: *const i32;
    unsafe {
        asm!(
            "lea ({1} + {2})(%rip), {0}",
            out(reg) addr,
            sym MY_GLOBAL,
            const offset_of!(MyStruct, b),
            options(att_syntax)
        );
    }
    assert_eq!(addr, &MY_GLOBAL.b as *const i32);
}
```
Having to use `offset_of!` to access a field is inconvenient. If we could pass
a pointer instead of being limited to a symbol name, then this would be no
issue as we could pass `&MY_GLOBAL.b`.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When writing assembly, you may use the `const` operand to insert a value that
was evaluated in const context. The following types are supported:

* Integers.
* Pointers. (To sized types.)
* Function pointers.

The `const` operand inserts the value directly into the inline assembly
verbatim. The value will be evaluated using const evaluation, which ensures
that the inserted value is known at compile time.

Note that when working with pointers in const evaluation, the pointers are
evaluated "symbolically". That is to say, in const eval, a pointer is a
symbolic value represented as an allocation and an offset. It's impossible to
turn a symbolic pointer into an integer during const eval. It's done this way
because when const evaluation runs, we don't yet know the address of globals.

The same caveat actually applies to assembly. We might not yet know the address
of a symbol or function when running the assembler or linker. For this reason,
linkers use similar symbolic math when working with pointers. This has
consequences for how you are allowed to use symbols in assembly.

The rest of the guide-level explanation will discuss what happens in practice
when you use the `const` operand in different scenarios. Note that all of these
examples also apply to the `sym` operand.

### Use in the `.text` section

Most commonly, instructions written in an inline assembly block will be stored
in the `.text` section. This is where your executable machine code is stored.

You can use the `const` operand to write a compile-time integer into the
machine code. For example:
```rs
use std::arch::asm;

fn main() {
    let a: i32;
    unsafe {
        asm!(
            "mov ${}, {:e}",
            const 42,
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("{}", a);
}
```
This will expand to a program where a `mov` instruction is used to write the
value 42 into a register, and the value of that register is then printed. The
value 42 is hard-coded into the mov instruction.

#### Position-independent code

When you use `const` with pointers rather than integers, you must think about
position-independent code.

Position-independent code is a special way of compiling machine code so that it
doesn't rely on the absolute address in memory it is stored at, and it is the
default on most Rust targets. This has various advantages:

* When loading shared libraries, you can store them at any unused address.
  There is no risk that two shared libraries need to be stored at the same
  location.
* It allows for address space layout randomization (ASLR), which is a
  mitigation that exploitation harder. The idea is that every time you run an
  executable, you store everything at a new address so that exploits cannot
  hardcode the address something is stored at.

However this means that the actual address of global variables is not yet known
at link-time. Since some instructions require the value to be known at
link-time, this can lead to linker errors when the `const` operand is used
incorrectly.

As an example of this going wrong, consider this code:
```rs
use std::arch::asm;

static FORTY_TWO: i32 = 42;

fn main() {
    let a: *const i32;
    unsafe {
        asm!(
            "mov ${}, {}",
            const &FORTY_TWO,
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("{:p}", a);
}
```
This will fail a linker error on most targets.

This error is because a `mov` instruction requires you to hard-code the actual
integer value into the instruction, but the address that `FORTY_TWO` will have
when you execute the code is not yet known when the assembly code is turned
into machine code.

Note that if you compiled this for a target such as `x86_64-unknown-none` which
does *not* use position independent code by default, then you will not get an
error because the absolute address of `FORTY_TWO` is known at compile-time, so
hard-coding it in `mov` is not an issue.

#### Relative values

Note that whether it fails doesn't just depend on the instruction, but also the
kind of expression the constant is used in. For example, consider this code:
```rs
use std::arch::asm;

static FORTY_TWO: i32 = 42;

fn main() {
    let a: *const i32;
    unsafe {
        asm!(
            "mov $({} - .), {}",
            const &FORTY_TWO,
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("{:p}", a);
}
```
```text
0x3cfb8
```
Here, the argument to `mov` is going to be `$(FORTY_TWO - .)` where the period
means "the address of this instruction". In this case, since `FORTY_TWO` and
the `mov` instruction are stored in the same object file, the linker is able to
compute the *offset* between the two addresses, even though it doesn't know the
absolute value of either address.

#### Rip-relative instructions

This comes up more often with rip-relative instructions, which are instructions
where the hard-coded value is relative to the instruction pointer (rip
register). For example, using the load-effective-address (lea) instruction:
```rs
use std::arch::asm;

static FORTY_TWO: i32 = 42;

fn main() {
    let a: *const i32;
    unsafe {
        asm!(
            "lea {}(%rip), {}",
            const &FORTY_TWO,
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("{:p}", a);
}
```
```text
0x562b445610ac
```
The above code creates a `lea` instruction that computes the value of `%rip`
plus some hard-coded offset. This allows the instruction to store the real
address of `FORTY_TWO` into `a` by hard-coding the offset between `FORTY_TWO`
and the lea instruction.

This kind of rip-relative instruction exists on basically every architecture.

### Symbols from dynamically loaded libraries

When you pass a pointer value to a symbol from a dynamically loaded library,
then it's not possible to use either absolute or relative addresses to access
it. The address is truly not known until runtime. This is for several reasons:

* The location at which the library is loaded is not known until runtime.
* Even if you knew the location of the library, the library could have been
  recompiled, so you don't even know the offset of the symbol in the library
  until runtime.

When you use the `const` operand with a pointer to a symbol from a dynamically
loaded library, you must use the symbol in one of the few contexts where this
is permitted. The simplest example of this is the `call` instruction:
```rs
use std::arch::asm;

fn main() {
    let exit_code: i32 = 42;

    unsafe {
        asm!(
            "call {}",
            const libc::exit,
            in("rdi") exit_code,
            options(att_syntax,noreturn),
        );
    }
}
```
In this scenario, the linker will expand `call` to different things depending
on where the symbol comes from and the platform. For example, on Linux, if you
`call` a symbol from another library, it uses a mechanism called the procedure
linkage table (PLT). Usually, the way this works is that instead of calling
`libc::exit` directly, it will call a dummy function in the PLT (which has a
constant offset from the `call` instruction). The dummy function will jump to
the real `libc::exit` function with the help of the dl loader.

Another scenario is global variables that are not functions. At least on Linux,
a global offset table (GOT) is used. Basically, the idea is that you are going
to store a big array of pointers called the GOT, and your executable or library
will include instructions to the linker (called relocations) that tell the
linker to replace each pointer with the address of a given symbol. Since the
GOT has a known fixed offset from your machine code, you can look up the
address of any symbol through the GOT.
```rs
use libc::FILE;
use std::arch::asm;

unsafe extern "C" {
    static stdin: *const FILE;
}

fn main() {
    // The GOT has a pointer of type `*const *const FILE` that points
    // to the real stdin global. This asm code will load the address
    // of that GOT entry into `a`.
    let a: *const *const *const FILE;
    unsafe {
        asm!(
            "leaq {}@GOTPCREL(%rip), {}",
            const &stdin,
            out(reg) a,
            options(att_syntax),
        );
    }
    // Check that dereferencing the GOT entry gives the address of
    // stdin.
    println!("offset: {}", unsafe { (&raw const stdin).byte_offset_from(*a)});
}
```
```text
offset: 0
```
Here, the `@GOTPCREL` directive tells the linker to create an entry in the GOT
containing the value before the @ sign, and the expression then evaluates to
the address of the GOT entry.

That said, you would usually not use the `@GOTPCREL` directive with the `const`
operand in machine code. The `@GOTPCREL` directive is mainly useful for loading
the address of the global into a register, and there is a significantly simpler
alternative for that: use the `in(reg)` operand instead of `const`.
```rs
use libc::FILE;
use std::arch::asm;

unsafe extern "C" {
    static stdin: *const FILE;
}

fn main() {
    let a: *const *const FILE;
    unsafe {
        asm!(
            "mov {}, {}",
            in(reg) &stdin,
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("offset: {}", unsafe { (&raw const stdin).byte_offset_from(a)});
}
```
```text
0
```
In this scenario, the compiler will compute the address of `stdin` before the
assembly block using whichever mechanism is most efficient for the given
symbol. In this case, that is a lookup using the GOT, but for a locally-defined
symbol it would not need a GOT lookup.

### Use in other sections

The `.text` section of the binary contains the executable machine code, and
this section is normally immutable. This ensures that if many programs load the
same shared library, the parts that constitute the `.text` section will be
identical across each copy, meaning that the same physical memory can be reused
for each copy of the library.

However, sections other than the `.text` section may not be immutable. For
example, the section that contains `static mut` variables is mutable. In this
case, we can make use of something called a *relocation*. This is a directive
to the dl loader, which tells it to *replace* a given location with the address
of a given symbol.

When you use the `const` operand to place a value in a custom section,
relocations are automatically used when necessary. This means that even though
the address of `FORTY_TWO` and `stdin` are not known in the below example, it's
still possible to store the addresses in static data:
```rs
use libc::FILE;
use std::arch::asm;

static FORTY_TWO: i32 = 42;

unsafe extern "C" {
    static stdin: *const FILE;
    static my_section_start: usize;
}

fn main() {
    // This asm block no longer computes a value at runtime. Instead,
    // it injects directives that instruct the assembler to create a
    // new section in the compiled binary and write data to it.
    #[allow(named_asm_labels)]
    unsafe {
        asm!(
            ".pushsection .my_data_section, \"aw\"",
            ".globl my_section_start",
            ".balign 8",
            "my_section_start:",
            ".quad {} - .", // period = address of this .quad
            ".quad {}",
            ".quad {}",
            ".popsection",
            const &FORTY_TWO,
            const &FORTY_TWO,
            const &stdin,
            options(att_syntax),
        );
    }

    let section: *const usize = unsafe { &my_section_start };

    let value1 = unsafe { *section.add(0).cast::<isize>() };
    let value2 = unsafe { *section.add(1).cast::<*const i32>() };
    let value3 = unsafe { *section.add(2).cast::<*const *const FILE>() };

    println!("{},{}", value1, unsafe { (&raw const FORTY_TWO).byte_offset_from(section) });
    println!("{:p},{:p}", value2, &raw const FORTY_TWO);
    println!("{:p},{:p}", value3, &raw const stdin);
}
```
```text
-75980,-75980
0x5a1f461700ac,0x5a1f461700ac
0x7da04bf026b0,0x7da04bf026b0
```
In this case, the asm block ends up creating a section containing three
integers:

* The offset from the section to the `FORTY_TWO` global.
* The address of the `FORTY_TWO` global.
* The address of the `stdin` global.

Only the first of these three values is actually a constant value, and if you
inspect the binary, the actual values in the section are going to be `-75980,
0, 0`. The two zeros are filled in when loading the program into memory based
on relocations emitted by the linker.

Note that if you try to use `stdin` with `{} - .` to make it relative, then
this will fail to compile because there is no relocation to insert a relative
address when the symbol is from a dynamically loaded library.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `const` operand has different behavior depending on the provided argument.
It accepts the following types:

* Any integer type.
* Raw pointers and references to sized types.
* Function pointers.

The argument is evaluated using const evaluation.

### Integer values

If the argument type is any integer type, then the value is inserted into the
asm block as plain text. This behavior exists on stable Rust today.

If the argument type is a raw pointer, but the value of the raw pointer is an
integer, then the behavior is the same as when passing an integer type. This
includes cases such as:

* `core::ptr::null()`
* `0xA000_000 as *mut u8`
* `core::ptr::null().wrappind_add(1000)`
* `core::ptr::without_provenance(1000)`

### Pointer values to a named symbol

When the argument type is a raw pointer, reference, or function pointer that
points at a named symbol, then the compiler will insert `symbol_name` into the
asm block as plain text. In this scenario, it is equivalent to using the `sym`
operand.

When the pointer was created from a named symbol, but is offset from the symbol
itself (e.g. it points at a field of the symbol), then the compiler will insert
`symbol_name+offset` (or `symbol_name-offset`) into the asm block as plain text.
In this scenario, using `{}` with a const operand is equivalent to writing
`{}+offset` (or `{}-offset`) with the `sym` operand.

The compiler may choose to emit the symbol name by inserting it into the asm
verbatim, or by using certain backend-specific operands (e.g. `'i'` or `'s'`),
depending on what the backend supports.

### Pointer values to an unnamed global

Not all globals are named. For example, when using static promotion to create a
variable stored statically, the location of the global has no name.

In this scenario, the compiler will generate a name for the symbol and emit
`symbol_name` or `symbol_name+offset` (or `symbol_name-offset`) using the newly
generated symbol, under the same rules as named symbols.

The compiler may choose any name for this symbol. The name may be chosen by
rustc and emitted to the backend as `symbol_name` or `symbol_name+offset` (or
`symbol_name-offset`), or rustc may pass the pointer to the backend using a
backend-specific operand (e.g. `'i'`) and let the backend choose the name.

### Coercions

Const parameters will be a coercion site for function pointers. This means that
when a function item is passed to a `const` argument, it will be coerced to a
function pointer. The same applies to closures without captures.

No other coercions will happen.

## Drawbacks
[drawbacks]: #drawbacks

The new operand supports every use-case that the `sym` operand supports (with
the possible exception of thread-locals). It may or may not make sense to emit
a warning if `const` is used in cases where `sym` could be used instead.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why extend the `const` operand

This RFC proposes to add pointer support to the existing `const` operand rather
than add a new operand or extend the `sym` operand. I think this makes sense,
since there are many other contexts where const-evaluated pointers work
together with the `const` keyword.

Extending the `sym` operand is not a workable solution because of the kind of
argument it takes. Currently, the `sym` operand takes a path, so if we extended
it to also support pointers, then `sym MY_GLOBAL` and `sym &MY_GLOBAL` would be
equivalent. Or worse, if `MY_GLOBAL` has a raw pointer type, then `sym
MY_GLOBAL` becomes ambiguous.

Adding a new operand is an option, but I don't think there is any reason to do
so. Using the name `const` for anything that can be evaluated during const
evaluation is entirely normal in Rust, even if the absolute address is not
known until runtime.

If we wish to choose a different name than `const` for the operand that takes a
pointer value, then we should be careful to pick a name that can not be
confused with the `memory` operand proposed in the future possibilities section
at the end of this RFC. The name `const` does not have this issue.

### What about wide pointers
[wide-pointers]: #what-about-wide-pointers

When passing a `&str` or `&[u8]` to an inline asm block, it could make sense to
treat this as the address of the given string. However, there is potential for
confusion with *interpolation*.

Interpolation is when a string is inserted verbatim into assembly. For example,
you could imagine having a string containing the name of a symbol and inserting
the string verbatim:
```rs
use std::arch::asm;

static FORTY_TWO: i32 = 42;

fn main() {
    let a: *const i32;
    unsafe {
        asm!(
            "mov ${}, {}",
            interpolate "FORTY_TWO",
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("{:p}", a);
}
```
Or even interpolating entire instructions:
```rs
use std::arch::asm;

static FORTY_TWO: i32 = 42;

fn main() {
    let a: *const i32;
    unsafe {
        asm!(
            "{}, {}",
            interpolate "mov $FORTY_TWO",
            out(reg) a,
            options(att_syntax),
        );
    }
    println!("{:p}", a);
}
```
To avoid confusion with this hypothetical interpolate operand, this RFC
proposes that wide pointers cannot be passed to the `const` operand. You must
do e.g. this:
```rs
const "my_string".as_ptr()
```
to insert a pointer to the string.

### Ambiguity in the expansion

Const evaluation is very restrictive about what you can do to a pointer. This
means that the pointer's provenance always unambiguously determines which
symbol should be used in the expansion.

Any future language features that introduce ambiguity here must address how
they affect the `const` operand. An example of such a feature would be casting
pointers to integers during const eval.

### What about codegen units

Rust may choose to split a crate into multiple codegen units to enable parallel
compilation. This is not an issue for this RFC because when the codegen units
are statically linked, the offsets between symbols from different units become
known constants. This allows the linker to resolve references between them
correctly.

### Implementation complexity

The implementation of this feature in rustc is straightforward. The compiler's
only responsibility is to perform const evaluation on the pointer and then
insert the resulting symbol and offset into the assembly string. All of the
complex logic for handling relocations and symbol resolution is handled by the
backend (LLVM) and the linker. Rustc does not need to implement any of this
logic itself.

### Large offsets and memory operands

Sarah brings up a concern about large offsets [on github](https://github.com/rust-lang/rust/issues/128464#issuecomment-2859580807).
In this concern, the assumption is that we are going to expand
```rs
asm!("lea rax, {P}", P = const &3usize);
```
to
```rs
asm!("lea rax, [rip + three_symbol]");
```
However this expansion is what you get when you use the memory operand `'m'`.
That is not the expansion used by this RFC. The `const` operand proposed by
this RFC corresponds to the `'i'` operand in C and *not* to the `'m'` operand.
The main difference here is that the `'m'` operand operates *on the place
behind the pointer*, whereas the `'i'` operand operates on the pointer value
itself.

This means that the code shared by Sarah [will fail with a linker error on most
Rust targets](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=c583db3a2aa7f007381eaec2029fd040)
because it's missing the `[rip + _]`. In assembly under Intel syntax, square
brackets is how you dereference an address. If you want the expansion that
Sarah used, you must instead write this:
```rs
asm!("lea rax, [rip + {P}]", P = const &3usize);
```
([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=684dc97aedb328b95c45b9725e1c0be5))

which uses the relatively simple expansion of inserting the symbol name
verbatim.

To summarize, the concern that Sarah shares about the `lea` instruction getting
mangled by LLVM is mostly relevant if we add a Rust equivalent to the `'m'`
operand, because that operand uses a much more complex expansion where you need
to understand the instruction that it is expanded into.

#### Why not add the memory operand instead?

The actual use-case that motivated this RFC is tracepoints in the Linux Kernel.
Here, we need to place a relative symbol into a section
```text
.pushsection .my_data_section, "aw"
.balign 8
.quad {} - .
.popsection
```
with `{}` being the address of a *field* in a `static`. The memory operand
cannot do this.

## Prior art
[prior-art]: #prior-art

When compared to C inline assembly, this feature is most similar to the `'i'`
operand. However, the `'i'` operand is less reliable to work with than what is
proposed in this RFC. For example, this C code:
```c
#include <stdio.h>

static const int FORTY_TWO = 42;

int main(void) {
    const int *a;

    __asm__ (
        "movabs %1 - ., %0"
        : "=r" (a)
	    : "i" (&FORTY_TWO)
    );

    printf("%p\n", (void *)a);

    return 0;
}
```
will have identical behavior to the `const` operand when it compiles. However,
in practice Clang will fail to compile this code on x86 targets using GOT
relocation, whereas GCC compiles it just fine.

Another difference is that C will accept runtime values to the `'i'` operand as
long as the compiler is able to optimize them to a constant value. That is to
say, whether the `'i'` operand compiles depends on compiler optimizations. This
means that in C, you can have a function that takes a pointer argument, and
pass it to the `'i'` operand. As long as the function is inlined and the caller
provided a constant value, this will compile.

To avoid having compiler optimizations (including inlining decisions!) affect
whether code compiles or not, this RFC proposes that the `const` operand
requires const evaluation even though this means that passing a pointer as a
function argument requires tricks such as this one:
```rs
use std::arch::asm;

trait HasGlobal {
    const PTR: *const Self;
}

static MY_I32: i32 = 42;
impl HasGlobal for i32 {
    const PTR: *const i32 = &MY_I32;
}

fn get_addr<T: HasGlobal>() -> *const T {
    let mut addr: *const T;
    unsafe {
        asm!(
            "lea {1}(%rip), {0}",
            out(reg) addr,
            const T::PTR,
            options(att_syntax)
        );
    }
    addr
}
```

## Future possibilities
[future-possibilities]: #future-possibilities

In the future, we may wish to consider adding other operands that Rust is
missing.

### Memory operand

It would make sense to add a Rust equivalent to the `'m'` operand, also called
the memory operand. The idea is that the operand takes a pointer argument, but
it expands to the place behind the pointer instead of the pointer itself. That
is to say, the operand contains an implicit dereference.

The memory operand is useful because it leaves significantly more flexibility
to the compiler / assembler. For example, if you use inline asm to read from a
global variable, then the compiler can choose one of several expansions:

* If the address of the global is known verbatim at link time, then the
  verbatim address may be hard-coded into the instruction.
* If the rip-relative address of the global is known, then a rip-relative
  instruction may be used instead.
* If the global is in another dynamic library, the compiler may load the
  address into a register before the asm block and insert that register in
  place of the operand.

That is, the operand is more limiting by not giving you access to the address
as a value, but that also makes it much more flexible. You usually do not need
to care about where the target symbol is defined with the memory operand.

Note that with the memory operand, const evaluation is not needed. If the
pointer is a runtime value, it will just be loaded into a register and the
operand will expand to something using that register.

### Interpolation

We could add an operand for interpolating a string into the assembly verbatim.
See [the section on wide pointers][wide-pointers] for more info.

### Formatting Specifiers

Similar to how `println!` uses format specifiers like `{:x}` or `{:?}` to change
how a value is printed, the `asm!` format string could be extended to support
specifiers for its operands. This would provide a more convenient way to request
architecture-specific formatting without requiring the user to write it
manually.

For example, a `pcrel` specifier could be introduced for program-counter-relative
addressing, used like `asm!("lea {0:pcrel}, rax", sym MY_GLOBAL)`. The specifier
(`:pcrel`) modifies how the operand is rendered. On x86, the behavior would be:

* For an integer (`const 123`), `{0:pcrel}` would expand to the integer value
  with a dollar sign: `$123`.
* For a symbol operand (`sym my_symbol`), `{0:pcrel}` would expand to
  `my_symbol(%rip)`.
* For an offset symbol operand (`const &MY_GLOBAL.field`), `{0:pcrel}` would
  expand to `(symbol+offset)(%rip)`.

This syntax could apply to both `sym` and `const` operands. This kind of
formatting can be quite useful due to assembly language quirks. For example, on
x86:

* On one hand, `symbol(%rip)` means `%rip + (symbol - %rip)` (where the part in
  parentheses is calculated at link time), so it is equal to just writing
  `symbol` except that the instruction uses rip-relative addressing.
* On the other hand, `100(%rip)` means `%rip + 100`, so it is *not* equal to
  `100`. The thing that actually means 100 in this context is `$100`.

Therefore, having a way to format into either `symbol(%rip)` or `$100` is quite
useful.

Note that `{:pcrel}` is an interesting middle ground between the bare
`const`/`sym` operand and the memory operand. On one hand, the expansion is
going to be architecture-specific, so it's a bit more complex than the
`symbol+offset` expansion. But unlike the memory operand, it does not need to
understand the context in which it is used within the asm block.
