# Day 2 Notes
* Part 3: Started at 6:07:21 on Day 2 for now

## Note from last time
* From Part 2, the difference in file size/offset was resolved.
  * 7:46:30 --> Brandon Falk realized he was on the wrong system.

## Code
* Pushing repo
  * Source code is available now on @gamozolabs github account!
  * Last code push from my own notetaking.
    * Will include fork in this repo for easy access later.
    * Currently have fork @dancwpark/fuzz_with_emus
    * Original is @gamozolabs/fuzzz_with_emus
  * [Brandon's commit](https://github.com/gamozolabs/fuzz_with_emus/tree/5fc5b6bec091cd7330b649c0b1d470076dd1a2e2)
    * Better than mine
* Code reorganization
  * main.rs
  * mmu.rs
  * emulator.rs
  * primitives.rs

## Plans
* Getting good/decent performance (~4mil cases) with the emulator
  * Without actual fuzzing yet
* VM/Emulator/MMu code is the bottleneck
  * Probably going to write a JIT
* Next day
  * JIT
  * Ramifications/consequences of all covered material.