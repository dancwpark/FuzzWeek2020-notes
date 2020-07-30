# Fuzz Week - Day 1
quick notes on non-code things


## Setting up target (objdump)
* Installed 2003 binutils
```bash
wget https://ftp.gnu.org/gnu/binutils/binutils-2.14.tar.gz
cd into it
./configure
make
``` 
  * Only up until it breaks :P
  * Still gives us enough binaries in /binutils
* Need older debugging symbols
```bash
make clean
CFLAGS="-O0 -g -gdwarf-2" ./configure 
make
```
* Copy ./objdump into objdump_fuzzer
## Set up corpus \& fuzzer
* Start working on a harness
  * Part of fuzzer that runs program and records crashes
  * Start by ignoring crashes
* Make directory called corpus (our inputs to objdump tool)
  * Make corpus of elf files
  * Take files from reg install
    * `cp /usr/bin/* .` find 
  * Prune out non-ELFs
    * `find * | xargs file | grep -v ELF | cut -d: -f1 | xargs rm`
* Write fuzz.py
* Moving to rust (?) because bottlenecking on python threading