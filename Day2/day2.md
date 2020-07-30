# day 2
raw and messy notes

## Finishing up the python fuzzer (fuzz.py)
* Make crash directory `crashes`
* from crash directory 
  * objdump -x crashes/[hash]
  * to recreate the crash

## Rewriting in Rust
* `new -> cargo new --bin objdump_rust_fuzzer`
* Quickly rewritten fuzzer is located in `main.rs`.

## Rust fuzzer speedup
* Looking into strace, there is a portion (~33 lines) that occurs
   before the file is even loaded.
  * Can this be removed from the fuzzing process?
  * Random `lseeks` to the same location multiple times
  * Reading stuff from memory
  * Solution: load file into ram to make the seeks go away
* Emulators and hypervisors the way to go because programs have
  too many globals. This limits speedup per process.

## Looking at AFL
* Compare our fuzzer to AFL
  * AFL default limits size of file that can be used as input...
    * changed in afl_fuzz.c :3
    * MAX_FILE => MAX_FILE+300
  * run afl using:
    * `./afl_fuzz -n -d -i inputs/ -o outputs/ -- ./objdump -a @@
    * inputs: copies corpus from day 1 notes
    * -n dumb mode (fuzz without instrumentation)
    * -d quick and dirty mode (skips deterministic steps)
    * @@ => replaced with input
* Limits file size to get "decent" speed/results
* Flips every bit in fuzz case before moving on... (Takes too long for not good coverage)
* AFL unique number of crashes means nothing really
* Industry uses libfuzzer and afl instead of 
  hypervisors/snapshots/emulation

## Writing a RISC-V emulator
* Write a RISC-V emulator to get rid of overhead!
* Using RISC-V toolchain from github.com/gamozolabs/riscv
* Need Clang 10 (what the above toolchain uses)
  * Put this into `/etc/apt/sources.list`
    * `deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main`
    * `deb-src http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main`
  * `sudo apt update`
  * Follow instructions at apt.llvm.org
    * llvm and clang
    * apt-get install clang-10 lldb-10 lld-10
* `make` in the RISC-V toolchain directory
  * It is possible on new installation that the compiler (clang) symlinks will
    be wonky.
    * Create symlinks, edit Makefile, or just install Clang alongside Clang 10.
* This builds custom toolchain for risv64 without multiplies and divide.
* `cd test_app; make` -> To build the test_app
* Can run the compiled test_app in qemu
  * `sudo apt install qemu-user`
* Jeez, qemu doesn't come with risc-v support when using precompiled binary in Ubuntu
  * Just spun up a Debian VM instead of trying to build from source
* Parts of the EMULATOR
  * Need to start with memory management system for the emulator
    * SMMU - soft memory management unit
    * Have process directly use the memory in the process
  * Heep track of modified memory -> when reset, only clear memory that was modified!
  * Emulator will be deterministic