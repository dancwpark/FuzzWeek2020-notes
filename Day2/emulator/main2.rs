use std::path::Path;

const PERM_READ: u8 = 1 << 0;
const PERM_WRITE: u8 = 1 << 1;
const PERM_EXEC: u8 = 1 << 2;
const PERM_RAW: u8 = 1 << 3;

pub unsafe trait Primitive: Default + Clone + Copy {}
unsafe impl Primitive for u8    {}
unsafe impl Primitive for u16   {}
unsafe impl Primitive for u32   {}
unsafe impl Primitive for u64   {}
unsafe impl Primitive for u128  {}
unsafe impl Primitive for usize {}
unsafe impl Primitive for i8    {}
unsafe impl Primitive for i16   {}
unsafe impl Primitive for i32   {}
unsafe impl Primitive for i64   {}
unsafe impl Primitive for i128  {}
unsafe impl Primitive for isize {}

/// Block size used for resetting and tracking memory which has
/// been modified.
/// The larger this is, the fewer but more expensive memcpys() need to occur,
/// the small, the greater but less expensive memcpys() need to occur.
/// It seems the sweet spot is often 128-2096 bytes
const DIRTY_BLOCK_SIZE: usize = 4096;

/// A permissions byte which corresponds to a memory byte and defines
/// the permissions it has
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Perm(pub u8);

/// A guest virtual address
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

/// An isolated memory space
pub struct Mmu {
    /// Block of memory for this address space
    /// Offset 0 corresponds to address 0 in the guest address space
    memory: Vec<u8>,

    /// Holds the permission bytes for the corresponding byte in memory
    permissions: Vec<Perm>,

    /// Tracks block indicies in `memory` which are dirty
    dirty: Vec<usize>,

    /// Tracks which parts of memory have been dirtied
    dirty_bitmap: Vec<u64>,

    /// Current base address of the next allocation
    cur_alc: VirtAddr,
}

impl Mmu {
    /// Create a new memory space which can hold `size` bytes
    pub fn new(size: usize) -> Self {
        Mmu {
            memory:       vec![0; size],
            permissions:  vec![Perm(0); size],
            dirty:        Vec::with_capacity(size / DIRTY_BLOCK_SIZE + 1),
            dirty_bitmap: vec![0u64; size / DIRTY_BLOCK_SIZE / 64 + 1],
            cur_alc:      VirtAddr(0x10000),
        }
    }
    
    /// Fork from an existing MMU
    fn fork(&self) -> Self {
        let size = self.memory.len();
        Mmu {
            memory:       self.memory.clone(),
            permissions:  self.permissions.clone(),
            dirty:        Vec::with_capacity(size / DIRTY_BLOCK_SIZE + 1),
            dirty_bitmap: vec![0u64; size / DIRTY_BLOCK_SIZE / 64 + 1],
            cur_alc:      self.cur_alc.clone(),
        }

    }


    /// Restores memory back to the original state (eg. restores all dirty
    /// blocks to the state of `other`)
    pub fn reset(&mut self, other: &Mmu) {
        for &block in &self.dirty {
            // Get the start and end addresses of dirtied memory
            let start = block * DIRTY_BLOCK_SIZE;
            let end = (block + 1) * DIRTY_BLOCK_SIZE;

            // Zero the bitmap. This is wide, but it's fine, we have to do
            // a 64-bit write anyways, no reason to compute the bit index
            self.dirty_bitmap[block / 64] = 0;
            
            // Restore memory state
            self.memory[start..end]
                .copy_from_slice(&other.memory[start..end]);
            
            // Restore permissions
            self.permissions[start..end]
                .copy_from_slice(&other.permissions[start..end]);
        }

        // Clear the dirty list
        self.dirty.clear();
    }
    
    /// Allocate a region of memory as RW in the address space
    pub fn allocate(&mut self, size: usize) -> Option<VirtAddr> {
        // 16-byte align the allocation
        let align_size = (size + 0xf) & !0xf; // make align/cache friendly?

        // Get the current allocation base
        let base = self.cur_alc;
        
        if base.0 >= self.memory.len() {
            return None;
        }

        // update the allocation size
        self.cur_alc = VirtAddr(self.cur_alc.0.checked_add(align_size)?);

        // Could nto satisfy allocation wihtout going oom
        if self.cur_alc.0 > self.memory.len() {
            return None;
        }
        
        // Mark the memory as uninitialized and writable
        self.set_permissions(base, size, Perm(PERM_RAW | PERM_WRITE));

        Some(base)
    }
    
    /// Apply permissions to a region of memory
    pub fn set_permissions(&mut self, addr: VirtAddr, size: usize, 
                           perm: Perm) -> Option<()> {
        
        // Apply permissions
        // base to end, add size to end, overflow -> error, set perm
        //  for each item 
        self.permissions.get_mut(addr.0..addr.0.checked_add(size)?)?
            .iter_mut().for_each(|x| *x = perm);

        Some(())
    }
    
    /// Write the bytes from `buf` into `addr`
    pub fn write_from(&mut self, addr: VirtAddr, buf: &[u8]) -> Option<()> {
        let perms = 
            self.permissions.get_mut(addr.0..addr.0.checked_add(buf.len())?)?;
        
        // Check permissions
        let mut has_raw = false;
        if !perms.iter().all(|x| {
            has_raw |= (x.0 & PERM_RAW) != 0;
            (x.0 & PERM_WRITE) != 0
        }) {
            return None;
        }
        self.memory.get_mut(addr.0..addr.0.checked_add(buf.len())?)?
            .copy_from_slice(buf);

        // compute dirty bit blocks
        let block_start = addr.0 / DIRTY_BLOCK_SIZE;
        let block_end = (addr.0 + buf.len()) / DIRTY_BLOCK_SIZE;
        for block in block_start..=block_end {
            // Determine the bit map position of the dirty block
            let idx = block_start / 64;
            let bit = block_start % 64;

            // Check if the block is not dirty
            if self.dirty_bitmap[idx] & (1 << bit) == 0 {
                // Block is not dirty, add it to the dirty list
                self.dirty.push(block);

                // Update the dirty bitmap
                self.dirty_bitmap[idx] |= 1 << bit;
            }
        } 


        // Update RAW bits
        if has_raw {
            perms.iter_mut().for_each(|x| {
                if (x.0 & PERM_RAW) != 0 {
                    // Mark memory as readable
                    *x = Perm(x.0 | PERM_READ);
                }
            });
        } 
        Some(())

    }
    /// Read the memory at `addr` into `buf`
    /// This function checks to see if all bits in `exp_perms` are set in
    /// the permissions bytes. If this is zero, we ignore permissions entirely.
    pub fn read_into_perms(&self, addr: VirtAddr, buf: &mut [u8], exp_perms:Perm) 
        -> Option<()> {
        let perms = 
            self.permissions.get(addr.0..addr.0.checked_add(buf.len())?)?;
        
        // Check permissions
        if exp_perms.0 != 0 && 
                !perms.iter().all(|x| (x.0 & exp_perms.0) == exp_perms.0) {
            return None;
        }
 

        buf.copy_from_slice(
            self.memory.get(addr.0..addr.0.checked_add(buf.len())?)?);
        Some(())

    }

    /// Read the memory at `addr` into `buf`
    pub fn read_into(&self, addr: VirtAddr, buf: &mut [u8]) -> Option<()> {
            self.read_into_perms(addr, buf, Perm(PERM_READ))
    }

    /// Read a type `T` at `addr` expecting `exp_perms`
    pub fn read_perms<T: Primitive>(&mut self, addr: VirtAddr, 
                                    exp_perms: Perm) -> Option<T> {
        let mut tmp = [0u8; 16];
        self.read_into_perms(addr, &mut tmp[..core::mem::size_of::<T>()],
                             exp_perms)?;
        Some(unsafe { core::ptr::read_unaligned(tmp.as_ptr() as *const T) })
    }
    
    /// Read a type `T` at `addr`
    pub fn read<T: Primitive>(&mut self, addr: VirtAddr) -> Option<T> {
        self.read_perms(addr, Perm(PERM_READ))
    }

    /// Write a val `T` at `addr`
    pub fn write<T: Primitive>(&mut self, addr: VirtAddr, val: T) 
        -> Option<()> {
        let tmp = unsafe {
            core::slice::from_raw_parts(&val as *const T as *const u8,
                                        core::mem::size_of::<T>())
        };
        self.write_from(addr, tmp) 
    }


}

/// All the states of the emulated system
pub struct Emulator {
    /// Memory for the emulator
    pub memory: Mmu,

    /// All RV64i registers
    registers: [u64; 33],
}

/// 64bit RISCV registers
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum Register {
    Zero = 0,
    Ra,
    Sp,
    Gp,
    Tp,
    T0,
    T1, 
    T2,
    S0,
    S1,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S2, 
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    T3,
    T4,
    T5,
    T6,
    Pc,
}

impl From<u32> for Register {
    fn from(val: u32) -> Self {
        assert!(val < 32);
        unsafe { core::ptr::read_unaligned(&(val as usize) as *const usize
                                            as *const Register) }
    }
}

impl Emulator {
    /// Creates a new emulator with `size` bytes of memory
    pub fn new(size: usize) -> Self {
        Emulator {
            memory: Mmu::new(size),
            registers: [0; 33],
        }
    }
    
    // Fork an emulator into a new emulator which will diff from the original
    pub fn fork(&self) -> Self {
        Emulator {
            memory: self.memory.fork(),
            registers: self.registers.clone(),
        }
    }

    /// Reset the state of `self` to `other`assuming that `self` is 
    /// forked off of `other`. If it is not, the results are invalid.
    pub fn reset(&mut self, other: &Self) {
        // Reset memory state 
        self.memory.reset(&other.memory);

        // Reset register state
        self.registers = other.registers;
    }

    /// Get a register from the guest
    pub fn reg(&mut self, register: Register) -> u64 {
       self.registers[register as usize]
    }
    
    /// Set a register in the guest
    pub fn set_reg(&mut self, register: Register, val: u64) {
       self.registers[register as usize] = val;
    }
    

    /// Load a file into the emulator address space using the sections as
    /// described.
    pub fn load<P: AsRef<Path>>(&mut self, filename: P, sections: &[Section]) -> Option<()> {
        // Read the input file
        let contents = std::fs::read(filename).ok()?;
        
        // Go through each section and load it
        for section in sections {
            // Set memory to writable
            self.memory.set_permissions(section.virt_addr, section.mem_size,
                                        Perm(PERM_WRITE))?; 
            // Write in the original file contents
            self.memory.write_from(section.virt_addr,
                        contents.get(section.file_off..
                        section.file_off.checked_add(section.file_size)?)?)?;

            // Write in any padding with zeros
            if section.mem_size > section.file_size {
                let padding = vec![0u8; section.mem_size - section.file_size];
                self.memory.write_from(
                    VirtAddr(section.virt_addr.0.checked_add(section.file_size)?),
                    &padding)?;
            }

            // Demote permissions to originals
            self.memory.set_permissions(section.virt_addr, section.mem_size,
                                        section.permissions)?; 

            // Update the allocator beyond any sections we load
            self.memory.cur_alc = VirtAddr(
                std::cmp::max(
                    self.memory.cur_alc.0,
                    (section.virt_addr.0 + section.mem_size + 0xf) & !0xf 
                )); // !0xf for rounding to nearest boundary
        }

        // print!("{:#x?}\n", self.memory.cur_alc);

        Some(())
    }

    pub fn run(&mut self) -> Option<()>{
        loop {
            // Get the current program counter 
            let pc = self.reg(Register::Pc);
            let inst : u32 = self.memory.read_perms(VirtAddr(pc as usize), 
                                                    Perm(PERM_EXEC))?;
            // Extract the opcode form the instruction
            let opcode = inst & 0b1111111;
            match opcode {
                0b0110111 => {
                    // LUI
                    let inst = Utype::from(inst);

                    self.set_reg(inst.rd, inst.imm as i64 as u64);
                }
                0b0010111 => {
                    // AUIPC
                    let inst = Utype::from(inst);
                    self.set_reg(inst.rd,
                                 (inst.imm as i64 as u64).wrapping_add(pc));
                }
                0b1101111 => {
                    // JAL
                    let inst = Jtype::from(inst);
                    self.set_reg(inst.rd, pc.wrapping_add(4));
                    self.set_reg(Register::Pc, 
                                 pc.wrapping_add(inst.imm as i64 as u64));
                }

                _ => unimplemented!("Unhandled opcode {:#09b}\n", opcode),
            }

            
            // Udate PC to the next instruction
            self.set_reg(Register::Pc, pc.wrapping_add(4));
        }

        Some(())
    }
}

struct Jtype {
    imm: i32,
    rd: Register
}

impl From<u32> for Jtype  {
    fn from(inst: u32) -> Self {
        let imm = inst & !0xfff;

        let imm20 = (imm >> 19) & 1;
        let imm101 = (imm >> 9) & 0b1111111111;
        let imm11 = (imm >> 8) & 1;
        let imm1912 = (imm >> 0) & 0b11111111;

        let imm = (imm20 << 20) | (imm1912 << 12) | (imm11 << 11) | 
            (imm101 << 1);
        let imm = ((imm as i32) << 11) >> 11;

        Jtype {
            imm: imm,
            rd: Register::from((inst >> 7) & 0b11111),
        }
    }
}

struct Utype {
    imm: i32,
    rd: Register
}

impl From<u32> for Utype  {
    fn from(inst: u32) -> Self {
        Utype {
            imm: (inst & !0xfff) as i32,
            rd: Register::from((inst >> 7) & 0b11111),
        }
    }
}

pub struct Section {
    pub file_off: usize,
    pub virt_addr: VirtAddr,
    pub file_size: usize,
    pub mem_size: usize,
    pub permissions: Perm, 
}

fn main() {
    //let mut 

    let mut emu = Emulator::new(32 * 1024 * 1024);
    emu.load("./test_app", &[
            Section{
                file_off: 0x000000000000000,   
                virt_addr: VirtAddr(0x000000000010000),   
                file_size: 0x000000000000190,   
                mem_size: 0x000000000000190,
                permissions: Perm(PERM_READ),
            },
            Section{
                file_off: 0x000000000000190,   
                virt_addr: VirtAddr(0x000000000011190),   
                file_size: 0x000000000002598,   
                mem_size: 0x000000000002598,
                permissions: Perm(PERM_EXEC),
            },
            Section{
                file_off: 0x000000000002728,   
                virt_addr: VirtAddr(0x000000000014728),   
                file_size: 0x0000000000000f8,   
                mem_size: 0x000000000000750,
                permissions: Perm(PERM_READ | PERM_WRITE),
            },

         ]).expect("Failed to load test application into address space");

    // Set the program entry point
    emu.set_reg(Register::Pc, 0x11190);
   
    emu.run().expect("Failed to execute emulator");
}
