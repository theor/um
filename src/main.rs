#![feature(raw)]

extern crate executable_memory;

use std::collections::HashMap;
use std::fs::File;

#[derive(Debug)]
struct Machine {
    pub regs: [u32; 8],
    pub arrays: HashMap<u32, Vec<u32>>,
    pub pc: usize,
}

use std::fmt;
impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  pc {}", self.pc)?;
        for i in 0..8 {
            write!(f, " reg{}:{}", i, self.regs[i])?;
        }
        write!(f, "")
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            regs: [0u32; 8],
            arrays: std::default::Default::default(),
            pc: 0,
        }
    }

    pub fn load_program(&mut self, program: &[u32]) {
        use std::iter::FromIterator;
        self.arrays
            .insert(0, Vec::from_iter(program.iter().cloned()));
    }
    fn reg(&mut self, i: u32) -> u32 {
        self.regs[i as usize]
    }

    fn reg_mut(&mut self, i: u32) -> &mut u32 {
        use std::ops::IndexMut;
        self.regs.index_mut(i as usize)
    }

    fn array(&self, i: u32) -> &Vec<u32> {
        self.arrays.get(&i).unwrap()
    }

    fn array_mut(&mut self, i: u32) -> &mut Vec<u32> {
        if !self.arrays.contains_key(&i) {
            // self.arrays.con
        }
        self.arrays.get_mut(&i).unwrap()
    }

    pub fn step(&mut self) {
        let line = (self.array(0))[self.pc];
        let op = line >> 28;
        let a = (line >> 6) & 7;
        let b = (line >> 3) & 7;
        let c = (line >> 0) & 7;
        println!("  line {:032b} op {:04b} a{:b} b{:x} c{:x}", line, op, a, b, c);
        println!("{}", self);
        self.pc += 1;

        match op {

            //The register A receives the value in register B,
            //unless the register C contains 0.
            0 => {
                println!("Conditional Move");
                if a != 0 {
                    self.regs[a as usize] = self.regs[b as usize];
                }
            }
            //The register A receives the value stored at offset
            //in register C in the array identified by B.
            1 => {
                println!("Array Index");
                self.regs[a as usize] = self.array(b)[c as usize];
            }

            //The array identified by A is amended at the offset
            //in register B to store the value in register C.
            2 => {
                println!("Array Amendment");
                self.array_mut(a)[b as usize] = self.regs[c as usize];
            }

            //The register A receives the value in register B plus
            //the value in register C, modulo 2^32.
            3 => {
                println!("Addition");
                *self.reg_mut(a) = self.reg(b) + self.reg(c);
            }

            //The register A receives the value in register B times
            //the value in register C, modulo 2^32.
            4 => {
                println!("Multiplication");
            }

            //The register A receives the value in register B
            //divided by the value in register C, if any, where
            //each quantity is treated treated as an unsigned 32
            //bit number.
            5 => {
                println!("Division");
            }

            6 => {
                println!("Not-And");
            }
 //   The universal machine stops computation.
            7 => {
                println!("Halt");
            }
//   A new array is created with a capacity of platters
            //   commensurate to the value in the register C. This
            //   new array is initialized entirely with platters
            //   holding the value 0. A bit pattern not consisting of
            //   exclusively the 0 bit, and that identifies no other
            //   active allocated array, is placed in the B register.
            8 => {
                println!("Allocation");
            } 
            //   The array identified by the register C is abandoned.
            //   Future allocations may then reuse that identifier.
            9 => {
                println!("Abandonment");
            } 
            //   The value in the register C is displayed on the console
            //   immediately. Only values between and including 0 and 255
            //   are allowed.
            10 => {
                println!("Output");
            } 
            //   The universal machine waits for input on the console.
            //   When input arrives, the register C is loaded with the
            //   input, which must be between and including 0 and 255.
            //   If the end of input has been signaled, then the
            //   register C is endowed with a uniform value pattern
            //   where every place is pregnant with the 1 bit.
            11 => {
                println!("Input");
            } 
            //   The array identified by the B register is duplicated
            //   and the duplicate shall replace the '0' array,
            //   regardless of size. The execution finger is placed
            //   to indicate the platter of this array that is
            //   described by the offset given in C, where the value
            //   0 denotes the first platter, 1 the second, et
            //   cetera.//   The '0' array shall be the most sublime choice for
            //   loading, and shall be handled with the utmost
            //   velocity.
            12 => {
                println!("Load Program");
                let arrb = self.arrays.get(&b);
                match arrb {
                    Some(arrb) => {
                        *self.array_mut(0) = arrb.to_vec();
                        self.pc = c as usize;
                    },
                    _ => (),
                }
            } 
            //    The value indicated is loaded into the register A
            //   forthwith.
            13 => {
                let a = op & 0x0E00_0000 >> 25;
                let value = op & 0x01FF_FFFF;
                println!("Orthography A:{} Value:{}", a, value);
                self.regs[a as usize] = value; 
            }
            _ => panic!("unknown op {}", op),
        }
    }
}

use executable_memory::ExecutableMemory;
use std::mem;

trait ExecutableMemoryExt {
    fn copy_from_slice_at(&mut self, index: usize, src: &[u8]);
    fn fill(&mut self, content: u8);
}

impl ExecutableMemoryExt for ExecutableMemory {
    fn copy_from_slice_at(&mut self, index: usize, src: &[u8]) {
        self.as_slice_mut()[index..index + src.len()].copy_from_slice(src)
    }
    fn fill(&mut self, content: u8) {
        unsafe {
            std::ptr::write_bytes(self.as_ptr(), content, self.len());
        }
    }
}

fn from_bytes<'a>(buf: &'a [u8]) -> Vec<u32> {
    let mut v = Vec::with_capacity(buf.len() / 4);
    for x in buf.chunks_exact(4) {
        
        let mut a: [u8; 4] = Default::default();
        a.copy_from_slice(x);
        let u: u32 = u32::from_be_bytes(a);
        // println!("{:x?} {:x}", x, u);
        v.push(u);
    }
    v
}

fn main() {
    let mut m = Machine::new();

    for arg in std::env::args().skip(1) {
        println!("{}", arg);
        match std::fs::File::open(arg) {
            Ok(mut f) => {
                use std::io::Read;
                let mut content = Vec::new();
                f.read_to_end(&mut content).unwrap();
                let content_u32: Vec<u32> = from_bytes(content.as_slice());
                m.load_program(content_u32.as_slice());
                // println!("{:x?}", m.array(0));
                break;
            }
            _ => continue,
        }
    }

    loop {
        m.step();
    }

    // let mut memory = ExecutableMemory::default(); // Page size 1
    // memory.fill(0xc3);
    // // x86_64
    // let slice = &[0x48, 0xC7, 0xC0, 0x03, 0x00, 0x00, 0x00];
    // memory.copy_from_slice_at(0, slice);
    // // memory[0] = 0xb8;
    // // memory[1] = 0x00;
    // // memory[2] = 0xff;
    // // memory[3] = 0xff;
    // // memory[4] = 0xff;
    // // memory[5] = 0xc3;

    // let f: fn() -> u32 = unsafe {
    //     mem::transmute((&memory[0..6]).as_ptr())
    // };

    // // assert_eq!(f(), 4294967295);
    // println!("{:x}", f());
    println!("exit");
}
