# day 2
raw and messy notes

* Make crash directory `crashes`
* from crash directory 
  * objdump -x crashes/[hash]
  * to recreate the crash

* Rewriting in Rust
* new -> cargo new --bin objdump_rust_fuzzer

* Speedup in rust?
* Looking into strace, there is a portion ~33 lines that occure
   before the file is even loaded.
  * Can this be removed from fuzzing?
* random lseeks to the same time multiple times
* reading stuff
* SOL: load file into ram to make the seeks go away

* emulators and hypervisors the way to go because programs have
  too many globals. This limits speedup per process.

* afl
* limits file size
* flips every bit in fuzz case before moving on...
  * need to run in dumb mode (-d)
  * also (-n) for binary instrumentation (??)
* afl unique number of crashes means nothing really
* afl default limits size of file that can be used as input...
  * changed in afl_fuzz.c :3
    * MAX_FILE => MAX_FILE+300
* run afl using:
  * `./afl_fuzz -n -d -i inputs/ -o outputs/ -- ./objdump -a @@
  * inputs: copies corpus from day 1 notes
  * -n dumb mode (fuzz without instrumentation)
  * -d quick and dirty mode (skips deterministic steps)
  * @@ => replaced with input
* industry uses libfuzzer and afl instead of 
  hypervisors/snapshots/emulation

* emulator??
* get rid of overhead!
* risc-v and unix emulator
* using risc-v from github.com/gamozolabs/riscv
* need clang 10
  * `deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main
     deb-src http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main`
  * put in /etc/apt/sources.list
  * apt update
  * follow instructions at apt.llvm.org
    * llvm and clang
    * apt-get install clang-10 lldb-10 lld-10
* `make`
* need lld-10
* This builds custom toolchain for risv64 without multiplies and divide
* cd test_app; make
* Can run in qemu
  * sudo apt install qemu-user
* Jeez, qemu doesn't come with risc-v support when installed using ubuntu and installed via apt
  * just spun up a debian vm instead of tryign to build from source...
* EMULATOR
  * need to start with memory management system for the emulator
    * smmu - soft memory management unit
    * have process directly use the memory in the process
  * keep track of modified memory -> when reset, only clear memory that was modified!
  * emulator will be deterministic