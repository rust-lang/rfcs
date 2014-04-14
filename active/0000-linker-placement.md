- Start Date: 2014-04-14
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Implement `#[link_placement]` attribute to specify where is static expected to be found in address space.

# Motivation

*(copied over from my other RFC)* In most of the cases, embedded hardware is configured with memory-mapped registers. Often a peripheral would have a moderately big number of registers to the point where tracking each one by its address is hard and prone to errors.

Given the unsafe nature of *T, I suggest to use &T instead for IO registers mapping.

# Detailed design

It is currently possible to provide placement information through a linker script:

example.rs
```rust
// this struct maps over a number of ioregs
pub struct UART {
  REG_A:  u32,
  REG_B:  u32,
  _pad_0: u32,
  REG_C:  u32,
}

impl UART {
  // given the nature of regs, we must use volatile access
  pub fn get_REG_A(&self) {
    unsafe { volatile_load(&(self.REG_A)) }
  }

  pub fn set_REG_A(&self, val: u32) {
    unsafe { volatile_store(&mut (self.REG_A), val) }
  }
  ...
}

// instantiate all peripherals over corresponding ioregs -> hard to read code
// pub static UART0 : *mut UART = 0xdeadbeef as *mut UART;

// ask the linker to provide correct address
extern {
  pub static mut UART0: UART;
}
```

script.ld
```
UART0 = 0xdeadbeef;
```

I propose to add `#[link_placement]` attribute that would allow to specify placement address directly in the code:

```rust
#[link_placement=0xdeadbeef]
pub static mut UART0: UART;
```

This way compiler knows symbol location and can provide better optimisations.

## Found in other languages

Proprietary C compilers for embedded systems are known to provide similar feature:

```c
static int reg @ 0xdeadbeef;
```

# Alternatives

N/A

# Unresolved questions

N/A
