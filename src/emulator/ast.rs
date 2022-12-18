use std::{collections::HashMap, str::FromStr};

use super::{*, lexer::{Token, Kind, UToken}, errorcontext::{ErrorContext, ErrorKind}, devices::IOPort};

struct TokenBuffer<'a> {
    index: usize,
    toks: Vec<UToken<'a>>
}
impl <'a> TokenBuffer<'a> {
    #[inline]
    pub fn new(toks: Vec<UToken<'a>>) -> Self {
        TokenBuffer {
            toks: toks,
            index: 0,
        }
    }
    #[inline]
    pub fn has_next(&self) -> bool {
        self.index < self.toks.len()
    }
    #[inline]
    pub fn advance(&mut self) {
        self.index += 1;
        while self.current().kind == Kind::White {
            self.index += 1;
        }
    }
    #[inline]
    pub fn next(&mut self) -> UToken<'a> {
        self.advance();
        self.toks[self.index].clone()
    }
    #[inline]
    pub fn current(&self) -> UToken<'a> {
        if self.has_next() {
            self.toks[self.index].clone()
        } else{
            Token {kind: Kind::EOF, str: ""}
        }
    }
    pub fn cur(&self) -> &UToken<'a> {
        if self.has_next() {
            &self.toks[self.index]
        } else {
            self.toks.last().unwrap()
        }
    }
}

pub struct Parser<'a> {
    buf: TokenBuffer<'a>,
    pub err: ErrorContext<'a>,
    pub ast: Program
}

fn remove_first(s: &str) -> Option<&str> {
    s.chars().next().map(|c| &s[c.len_utf8()..])
}

pub fn gen_ast<'a>(toks: Vec<UToken<'a>>) -> Parser {
    let mut err = ErrorContext::new();
    let mut ast = Program::new();
    let mut buf = TokenBuffer::new(toks);
    let mut p = Parser {buf, err, ast};

    while p.buf.has_next() {
        match p.buf.current().kind {
            Kind::Name => {
                match p.buf.current().str.to_lowercase().as_str() {
                    "imm" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Int(v) => Operand::Imm(v as u64), _ => {continue;} };
                        
                        p.ast.instructions.push(
                            Inst::IMM(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "mov" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        p.ast.instructions.push(
                            Inst::MOV(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "add" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        let op3 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::ADD(op1, op2, op3)
                        );
                        p.buf.advance();
                    },
                    "rsh" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::RSH(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "lod" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), Kind::Memory(v) => Operand::Mem(v as u64), _ => {continue;} };
                        p.ast.instructions.push(
                            Inst::LOD(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "str" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), Kind::Memory(v) => Operand::Mem(v as u64), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::STR(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "bge" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        let op3 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        
                        p.ast.instructions.push(
                            Inst::BGE(op1, op2, op3)
                        );

                        p.buf.advance();
                    },
                    "nor" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        let op3 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::NOR(op1, op2, op3)
                        );
                        p.buf.advance();
                    },
                    "inc" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::INC(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "dec" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::DEC(op1, op2)
                        );
                        p.buf.advance();
                    }
                    "hlt" => {
                        p.ast.instructions.push(
                            Inst::HLT
                        );
                        p.buf.advance();
                    },
                    "jmp" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::JMP(op1)
                        );
                        p.buf.advance();
                    },
                    "sub" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        let op3 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::SUB(op1, op2, op3)
                        );
                        p.buf.advance();
                    },
                    "nop" => {
                        p.ast.instructions.push(Inst::NOP);
                    },
                    "lsh" => {
                        let op1 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {continue;} };
                        let op2 = match p.buf.next().kind { Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}} };
                        p.ast.instructions.push(
                            Inst::LSH(op1, op2)
                        );
                        p.buf.advance();
                    },
                    "bits" => {
                        p.ast.headers.bits = match p.buf.next().kind { Kind::Int(v) => v as u64, _ => match p.buf.next().kind {Kind::Int(v) => v as u64, _ => continue} };
                        p.buf.advance();
                    },
                    "minreg" => {
                        p.ast.headers.minreg = match p.buf.next().kind {Kind::Int(v) => v as u64, _ => {continue;}};
                        p.buf.advance();
                    },
                    "minheap" => {
                        p.ast.headers.minheap = match p.buf.next().kind {Kind::Int(v) => v as u64, _ => {continue;}};
                        p.buf.advance();
                    },
                    "minstack" => {
                        p.ast.headers.minstack = match p.buf.next().kind {Kind::Int(v) => v as u64, _ => {continue;}};
                        p.buf.advance();
                    },
                    "out" => {
                        let a = match p.buf.next().kind {Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}}};
                        let b = match p.buf.next().kind {Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}}};

                        p.ast.instructions.push(Inst::OUT(a, b));
                        p.buf.advance();
                    },
                    "in" => {
                        let a = match p.buf.next().kind {Kind::Reg(v) => Operand::Reg(v), _ => continue,};
                        let b = match p.buf.next().kind {Kind::Reg(v) => Operand::Reg(v), _ => {match get_operand(&mut p) {Some(v) => v, None => continue,}}};
                        p.ast.instructions.push(Inst::IN(a, b));
                        p.buf.advance();
                    },
                    "psh" => {
                        let Some(op) = p.get_imm() else {continue};
                        p.assert_done();

                        p.ast.instructions.push(Inst::PSH(op));
                    },
                    "pop" => {
                        let Some(op) = p.get_reg() else {continue};
                        p.assert_done();

                        p.ast.instructions.push(Inst::POP(op));
                    },
                    "jmp" => {
                        let Some(op) = p.get_imm() else {continue};
                        p.assert_done();
                        
                        p.ast.instructions.push(Inst::JMP(op));
                    }
                    _ => { jsprintln!("Unhandled name: {:#?}", p.buf.current().str); p.buf.advance(); },
                }
            },
            Kind::Label => {
                match p.ast.labels.get(p.buf.current().str) {
                    Some(Label::Defined(_)) => jsprintln!("Redefined label: {}", p.buf.current().str),
                    Some(Label::Undefined(v)) => {
                        for i in v.iter() {
                            match p.ast.instructions[*i] {
                                _ => continue,
                            } // yeah changes in my fork idk how to impl too-late labels but this is good enough for now
                        }

                        jsprintln!("Defined label {} too late lol I didnt impl that", p.buf.current().str);
                    },
                    None => { p.ast.labels.insert(p.buf.current().str.to_string(), Label::Defined(p.ast.instructions.len())); },
                }
                p.buf.advance();
            },
            Kind::White | Kind::Comment | Kind::LF => p.buf.advance(),
            _ => { logprintln!("Unhandled token type: {:#?}", p.buf.current());  p.buf.advance(); },
        }
    }

    p
}

impl <'a> Parser<'a> {
    fn get_reg(&mut self) -> Option<Operand> {
        let Some(operand) = self.get_imm() else {
            self.err.error(self.buf.cur(), ErrorKind::InvalidOpperandType);
            return None;
        };
        match operand { // operant 👍 i think its d
            Operand::Reg(num) => {
                if num == 0 {
                    Some(Operand::Imm(0))
                } else {
                    Some(operand)
                }
            },
            _ => {
                self.err.error(&self.buf.cur(), ErrorKind::InvalidOpperandType);
                None
            },
        }
    }
    fn get_imm(&mut self) -> Option<Operand> {
        self.buf.advance();
        let result = get_operand(self);
        result
    }
    fn assert_done(&mut self) {
        self.buf.advance();
        match self.buf.current().kind {
            Kind::LF |  Kind::EOF => {},
            _ => {
                self.err.error(&self.buf.current(), ErrorKind::NotEnoughOpperands)
            }
        }
    }
}


fn get_operand(p: &mut Parser) -> Option<Operand> {
    match p.buf.current().kind {
        Kind::Reg(v) => Some(Operand::Reg(v)),
        Kind::Int(v) => Some(Operand::Imm(v as u64)),
        Kind::PortNum(v) => Some(Operand::Imm(v)),
        Kind::Port => {
            match IOPort::from_str(&p.buf.current().str[1..].to_uppercase()) {
                Ok(port) => {Some(Operand::Imm(port as u64))},
                Err(err) => {
                    p.err.error(&p.buf.current(), ErrorKind::UnknownPort);
                    jsprintln!("{:#}", err);
                    None
                } // TODO: report error
            }
        }
        Kind::Label  => Some(label_to_operand(&p.buf.current(), p)),
        _ => None
    }
}

#[derive(Debug, PartialEq)]
pub enum Label {
    Undefined(Vec<usize>),
    Defined(usize),
}

fn label_to_operand<'a>(tok: &UToken<'a>, p: &mut Parser) -> Operand {
    if (*tok).kind != Kind::Label {return Operand::Imm(0);}

    match p.ast.labels.get(tok.str) {
        Some(Label::Undefined(v)) => {
            let mut a = v.clone();
            a.push(p.ast.instructions.len());
            p.ast.labels.insert((*tok).str.to_string(), Label::Undefined(a));
            Operand::Label(tok.str.to_string())
        },
        Some(Label::Defined(v)) => Operand::Imm(*v as u64),
        None => {
            let mut a = Vec::new();
            a.push(p.ast.instructions.len());
            p.ast.labels.insert((*tok).str.to_string(), Label::Undefined(a));
            Operand::Label(tok.str.to_string())
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub headers: Headers,
    pub instructions: Vec<Inst>,
    pub labels: HashMap<String, Label>
}

impl Program {
    pub fn new() -> Self {
        Program { headers: Headers::new(), instructions: Vec::new(), labels: HashMap::new() }
    }
}

#[derive(Debug, Clone)] // cant copy because of the String
pub enum Operand {
    Imm(u64),
    Reg(u64),// it gets changed to immediates, try it out
    Mem(u64),
    Label(String),
}

// kind (imm, reg, mem, label) 1 byte
// start: 4/8bytes, capacity: 4/8bytes, length: 4/8bytes
// 8*4 = 32 bytes
// but it only has to be 16 bytes
// or even better, we could pack the bytes but that has other diadvantages
// unless... we put the opcodes in a sepperate buffer like this
// but idk what that will do for performance it could make it better it could not matter at all
// we should benchmark our different options
// cant we get it working and then optimize
struct EmuProgram {
    opcodes: Vec<Opcode>, // only 1 byte per instruction instead of the 8 bytes because we don't need to allign it with the immediate 
    // oh right the operands aren't just emmediates... 🤔
    // they can also be registers
    // maybe should just try and se how fast your current implementation is 😌
    immediates: Vec<u64>
}

enum Opcode {}

#[derive(Debug)]
pub struct Headers {
    pub bits: u64,
    pub minheap: u64,
    pub minstack: u64,
    pub minreg: u64
}

impl Headers {
    pub fn new() -> Self {
        Headers { bits: 8, minheap: 16, minstack: 16, minreg: 8 } // replace all r0 with 0
    }
}

#[derive(Debug, Clone)]
pub enum Inst {
    IMM(Operand, Operand),
    ADD(Operand, Operand, Operand),
    RSH(Operand, Operand),
    LOD(Operand, Operand),
    STR(Operand, Operand),
    BGE(Operand, Operand, Operand),
    NOR(Operand, Operand, Operand),
    MOV(Operand, Operand),
    INC(Operand, Operand),
    DEC(Operand, Operand),
    OUT(Operand, Operand),
    IN(Operand, Operand),
    HLT,
    
    PSH(Operand),
    POP(Operand),
    JMP(Operand),
    SUB(Operand, Operand, Operand),
    NOP,
    LSH(Operand, Operand),
}


// pub trait Port {
//     fn port_out(&mut self, data: u64);
//     fn port_in(&mut self) -> u64;
// }
