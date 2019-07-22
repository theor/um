#![feature(raw)]

#[macro_use] extern crate log;
extern crate pretty_env_logger;
extern crate executable_memory;

extern crate rustyline;
extern crate termcolor;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::collections::VecDeque;

#[derive(Debug)]
struct Machine {
    pub regs: [u32; 8],
    pub array0: Vec<u32>,
    pub arrays: Vec<Vec<u32>>,
    pub freelist: VecDeque<u32>,
    pub pc: usize,
    input_buffer: String,
}

use std::fmt;
impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "    pc {}", self.pc)?;
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
            array0: Vec::default(),
            arrays: vec![Vec::default(); 1],
            freelist: VecDeque::default(),
            pc: 0,
            input_buffer: String::default(),
        }
    }

    pub fn load_program(&mut self, program: &[u32]) {
        use std::iter::FromIterator;
        self.array0 = Vec::from_iter(program.iter().cloned());
    }
    fn reg(&self, i: u32) -> u32 {
        self.regs[i as usize]
    }

    fn reg_mut(&mut self, i: u32) -> &mut u32 {
        use std::ops::IndexMut;
        self.regs.index_mut(i as usize)
    }

    fn array(&self, i: u32) -> &Vec<u32> {
        if i == 0 { 
            &self.array0
        } else {
            self.arrays.get(i as usize).unwrap()
        }
    }

    fn array_mut(&mut self, i: u32) -> &mut Vec<u32> {
        if i == 0 { 
            &mut self.array0
        } else {
            self.arrays.get_mut(i as usize).unwrap()
        }
    }

/* 1st four:
080000D0 Cmov 3 2 0
300000C0 Add 3 0 0
D2000014 Ortho 0 2 4
D400005B Ortho 1 3 3

 */

    pub fn step(&mut self, out: &mut std::io::BufWriter<std::fs::File>, rl: &mut Editor<()>) -> bool {
        let line = self.array0[self.pc];
        let op = line >> 28;
        let a = (line >> 6) & 7;
        let b = (line >> 3) & 7;
        let c = (line >> 0) & 7;
        // println!("  line {:032b} op {:04b} a{:b} b{:x} c{:x}", line, op, a, b, c);
        use log::Level;

        if log_enabled!(Level::Debug) {
            trace!("{:08x} op {:04b} a{} b{} c{}", line, op, a, b, c);
        } else {
            debug!("{:08x}", line);
        }
        // print!("  ");
        self.pc += 1;

        match op {

            //The register A receives the value in register B,
            //unless the register C contains 0.
            0 => {
                trace!("Conditional Move ");
                if self.reg(c) != 0 {
                    self.regs[a as usize] = self.regs[b as usize];
                }
            }
            //The register A receives the value stored at offset
            //in register C in the array identified by B.
            1 => {
                info!("Array Index {:08X}", line);
                *self.reg_mut(a) = self.array(self.reg(b))[self.reg(c) as usize];
            }

            //The array identified by A is amended at the offset
            //in register B to store the value in register C.
            2 => {
                trace!("Array Amendment");
                let rb = self.reg(b);
                self.array_mut(self.reg(a))[rb as usize] = self.reg(c);
            }

            //The register A receives the value in register B plus
            //the value in register C, modulo 2^32.
            3 => {
                use std::num::Wrapping;
                trace!("Addition");
                *self.reg_mut(a) = (Wrapping(self.reg(b)) + Wrapping(self.reg(c))).0;
            }


            //The register A receives the value in register B times
            //the value in register C, modulo 2^32.
            4 => {
                use std::num::Wrapping;
                trace!("Multiplication");
                *self.reg_mut(a) = (Wrapping(self.reg(b)) * Wrapping(self.reg(c))).0;
            }

            //The register A receives the value in register B
            //divided by the value in register C, if any, where
            //each quantity is treated treated as an unsigned 32
            //bit number.
            5 => {
                trace!("Division");
                *self.reg_mut(a) = self.reg(b) / self.reg(c);
            }

            //  Each bit in the register A receives the 1 bit if
            //  either register B or register C has a 0 bit in that
            //  position.  Otherwise the bit in register A receives
            //  the 0 bit.
            6 => {
                trace!("Not-And");
                *self.reg_mut(a) = !(self.reg(b) & self.reg(c));
            }

            //   The universal machine stops computation.
            7 => {
                trace!("Halt");
                return false;
            }
            //   A new array is created with a capacity of platters
            //   commensurate to the value in the register C. This
            //   new array is initialized entirely with platters
            //   holding the value 0. A bit pattern not consisting of
            //   exclusively the 0 bit, and that identifies no other
            //   active allocated array, is placed in the B register.
            8 => {
                trace!("Allocation {:08X}", line);
                let size = self.reg(c);
                match self.freelist.pop_front(){
                    None => {
                        let new_array =  vec![0; size as usize];
                        trace!("  New Alloc size={} b={} {:?} count {}", size, self.arrays.len(), self.freelist, self.arrays.len());
                        *self.reg_mut(b) = self.arrays.len() as u32;
                        self.arrays.push(new_array);
                    },
                    Some(i) => {
                        assert!(i != 0);
                        *self.reg_mut(b) = i as u32;
                        // self.array_mut(i).resize(size as usize, 0);
                        *self.array_mut(i) = vec![0; size as usize];
                        assert_eq!(size as usize, self.array(i).len());
                        // println!("reuse at {} exp size {} act {}", i, size, self.array(i).len());
                        trace!("  REUSE Alloc size={} b={} {:?}", size, i, self.freelist);
                    },
                }
            } 
            //   The array identified by the register C is abandoned.
            //   Future allocations may then reuse that identifier.
            9 => {
                trace!("Abandonment {:08X}", line);
                self.freelist.push_back(self.reg(c));
                trace!("  Freelist: {:?}", self.freelist);
                // assert!(!self.arrays.remove(&self.reg(c)).is_none());
            } 
            //   The value in the register C is displayed on the console
            //   immediately. Only values between and including 0 and 255
            //   are allowed.
            10 => {
                trace!("Output");
                let chr: char = (self.reg(c) as u8).into();
                print!("{}", chr);
                std::io::stdout().flush().unwrap();               

                use std::io::Write;
                // out.write(&[chr as u8]).unwrap();
            } 
            //   The universal machine waits for input on the console.
            //   When input arrives, the register C is loaded with the
            //   input, which must be between and including 0 and 255.
            //   If the end of input has been signaled, then the
            //   register C is endowed with a uniform value pattern
            //   where every place is pregnant with the 1 bit.
            11 => {
                trace!("Input");

                if self.input_buffer.len() == 0 {
                    loop {
                        use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

                        let mut stdout = StandardStream::stdout(ColorChoice::Auto);
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
                        let readline = rl.readline(">> ");
                        stdout.reset().unwrap();

                        match readline {
                            Ok(ref read_line) if read_line.len() > 0 => {
                                rl.add_history_entry(read_line.as_str());
                                self.input_buffer = format!("{}\n", read_line);
                                break
                            },
                            Err(ReadlineError::Interrupted) => {
                                println!("CTRL-C");
                            },
                            Err(ReadlineError::Eof) => {
                                println!("CTRL-D");
                                return false;
                            },
                            _ => (),
                        }
                    }
                }

                let chr = self.input_buffer.remove(0);
                *self.reg_mut(c) = chr as u32;

        

                // use std::io::{self, Read};
                // let mut buf = [0];
                // io::stdin().read_exact(&mut buf).unwrap();
                // *self.reg_mut(c) = buf[0] as u32;

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
                trace!("Load Program");
                if self.reg(b) != 0 {
                    let arrb = self.array(self.reg(b));
                    *self.array_mut(0) = arrb.to_owned();
                }
                self.pc = self.reg(c) as usize;
            } 
            //    The value indicated is loaded into the register A
            //   forthwith.
            13 => {
                let a = (line >> 25) & 0b111;
                let value = line & 0x1FFFFFF;
                trace!("Orthography A:{} Value:{}", a, value);
                self.regs[a as usize] = value; 
            }
            _ => panic!("unknown op {}", op),
        }
        trace!("{}", self);
        true
    }
}

use executable_memory::ExecutableMemory;

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
        let word: u32 = ((x[0] as u32) << 24) | ((x[1] as u32) << 16) | ((x[2] as u32) << 8) | (x[3] as u32);
        // println!("{:x?} {:x}", x, u);
        v.push(word);
    }
    v
}

fn main() {
    pretty_env_logger::init();
    let mut m = Machine::new();

    let mut file = None;
    for arg in std::env::args().skip(1) {
        if arg.starts_with("-") {
            println!("{}", arg);
        } else {
            file = Some(arg.to_owned());
        }
    }

    if file.is_none() {
        return;
    }

    match std::fs::File::open(file.unwrap()) {
        Ok(mut f) => {
            use std::io::Read;
            let mut content = Vec::new();
            f.read_to_end(&mut content).unwrap();
            let content_u32: Vec<u32> = from_bytes(content.as_slice());
            
            m.load_program(content_u32.as_slice());
            println!("{}", m.array0.len());
        }
        _ => return,
    }

    use std::fs::OpenOptions;
    use std::io::BufWriter;

    // `()` can be used when no completer is required
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }


    let mut out = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("out.bin.um").map(|f| BufWriter::with_capacity(128, f)).unwrap();
// let mut i = 0;
    loop {
        // println!("{}", i);
        if !m.step(&mut out, &mut rl){
            break;
        }
        // i += 1;
        // if i >= 100 { break; }
    }
    rl.save_history("history.txt").unwrap();

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
