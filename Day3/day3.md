# Day 3
Writing a RISC-V JIT
Video Link: https://www.youtube.com/watch?v=ZAP3J86VUWI
Code Link: https://github.com/gamozolabs/fuzz_with_emus/tree/63d87824f676d0e43262c88b4004053aae119c1c

## Building a RISC-V toolchain
* Building a riscv-gnu-toolchain
  * Link: github.com/riscv/riscv-gnu-toolchain
  * Base (RV32i/64i) Integer Instruction Set (all others are extensions)
    * RISC-V does not come out of the box with MUL and DIV (this is an extension)
    * Same for atomic operations (both 32 and 64 bit) and float operations
  * Most people use RV64G
    * Base with MUL/DIV, atomic, single and double precision float
* Get source
* Get prereqs
* Build the linux toolchain
  * Breaks because needs A extension (atomics)
  * Instead, build against newlib
    * `./configur e--prefix=/opt/rv64i-newlib ...`

## Newlib (Red Hat newlib C Library)
* Very simplified with minimal number of instructions
  * Simplicity -> can use almost as an intermediate language
* "libc binding sort of thing with a reduced set of syscalls"
  * Developed for embedded devices


## Code stuff
* Edited emulator and fuzzer to work with newlib riscv toolchain
  * New syscall handling
* Need to rebuild target (objdump from binutils) for newlib
* `/opt/rv64i-newlibbin/riscv64-unknown-elf-gcc -W -Wall -Wstrict-prototypes -Wmissing-prototypes -g -O2 -o objdump objdump.o budemang.o prdbg.o rddbg.o debug.o stabs.o ieee.o rdcoff.o bucomm.o version.o filemode.o .../opcodes/.libs/libopcodes.a ../bfd/.libs/libbfd.a ../libiberty/libiberty.a ./..intl/libintl.a`
  * from inside `binutils-2.14/`


* NOTE: Stopped @ 3:53:07