use std::{collections::HashMap, str::FromStr, rc::Rc};

use super::{lexer::{Token, Kind, UToken}, errorcontext::{ErrorContext, ErrorKind}, devices::IOPort};

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
        while matches!(self.current().kind, Kind::White | Kind::Comment) {
            self.index += 1;
        }
    }
    #[inline]
    pub fn peek(&mut self) -> UToken<'a> {
        let mut a = self.index + 1;
        while matches!(self.toks[a].kind, Kind::White | Kind::Comment | Kind::LF) {
            a += 1;
        }
        self.toks[a].clone()
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
        } else {
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
    pub ast: Program,
    pub at_line: usize,
    pub macros: HashMap<&'a str, UToken<'a>>
}

pub fn gen_ast<'a>(toks: Vec<UToken<'a>>, src: Rc<str>) -> Parser<'a> {
    let err = ErrorContext::new();
    let ast = Program::new(src);
    let buf = TokenBuffer::new(toks);
    let mut p = Parser {buf, err, ast, at_line: 1, macros: HashMap::new() };

    let mut dw_lab_repl: HashMap<String, Vec<u64>> = HashMap::new();
    let mut dw_mem_repl: Vec<u64> = Vec::new();

    while p.buf.has_next() {
        match p.buf.current().kind {
            Kind::Name => {
                match p.buf.current().str.to_lowercase().as_str() {
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

                    "dw" => {
                        match p.buf.next().kind {
                            Kind::LSquare => {
                                while p.buf.has_next() {
                                    if matches!(p.buf.next().kind, Kind::RSquare) { break }
                                    let mut a = p.parse_dw(&mut dw_lab_repl, &mut dw_mem_repl);
                                    p.ast.memory.append(&mut a);
                                }
                            },
                            _ => {
                                let mut a = p.parse_dw(&mut dw_lab_repl, &mut dw_mem_repl);
                                p.ast.memory.append(&mut a)
                            }
                        }
                    },

                    "imm"     => inst(Inst::MOV(p.get_reg(), p.get_imm())           , &mut p),
                    "mov"     => inst(Inst::MOV(p.get_reg(), p.get_op())            , &mut p),
                    "add"     => inst(Inst::ADD(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "rsh"     => inst(Inst::RSH(p.get_reg(), p.get_op())            , &mut p),
                    "lod"     => inst(Inst::LOD(p.get_reg(), p.get_mem())           , &mut p),
                    "str"     => inst(Inst::STR(p.get_mem(), p.get_op())            , &mut p),
                    "bge"     => inst(Inst::BGE(p.get_jmp(), p.get_op(), p.get_op()), &mut p),
                    "nor"     => inst(Inst::NOR(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "inc"     => inst(Inst::INC(p.get_reg(), p.get_op())            , &mut p),
                    "dec"     => inst(Inst::DEC(p.get_reg(), p.get_op())            , &mut p),
                    "hlt"     => inst(Inst::HLT                                     , &mut p),
                    "sub"     => inst(Inst::SUB(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "nop"     => inst(Inst::NOP                                     , &mut p),
                    "lsh"     => inst(Inst::LSH(p.get_reg(), p.get_op())            , &mut p),
                    "out"     => inst(Inst::OUT(p.get_port(), p.get_op())           , &mut p),
                    "in"      => inst(Inst::IN (p.get_reg(), p.get_port())          , &mut p),
                    "psh"     => inst(Inst::PSH(p.get_op())                         , &mut p),
                    "pop"     => inst(Inst::POP(p.get_reg())                        , &mut p),
                    "jmp"     => inst(Inst::JMP(p.get_jmp())                        , &mut p),
                    "neg"     => inst(Inst::NEG(p.get_reg(), p.get_op())            , &mut p),
                    "and"     => inst(Inst::AND(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "or"      => inst(Inst::OR (p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "not"     => inst(Inst::NOT(p.get_reg(), p.get_op())            , &mut p),
                    "nand"    => inst(Inst::NAND(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "cpy"     => inst(Inst::CPY(p.get_mem(), p.get_mem())           , &mut p),
                    "mlt"     => inst(Inst::MLT(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "div"     => inst(Inst::DIV(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "mod"     => inst(Inst::MOD(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "abs"     => inst(Inst::ABS(p.get_reg(), p.get_op())            , &mut p),
                    "llod"    => inst(Inst::LLOD(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "lstr"    => inst(Inst::LSTR(p.get_op(), p.get_op(), p.get_op()), &mut p),
                    "sdiv"    => inst(Inst::SDIV(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "sete"    => inst(Inst::SETE(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "setne"   => inst(Inst::SETNE(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "setg"    => inst(Inst::SETG(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "setge"   => inst(Inst::SETGE(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "setl"    => inst(Inst::SETL(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "setle"   => inst(Inst::SETLE(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "xor"     => inst(Inst::XOR(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "xnor"    => inst(Inst::XNOR(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "bne"     => inst(Inst::BNE(p.get_op(), p.get_op(), p.get_op()) , &mut p),
                    "bre"     => inst(Inst::BRE(p.get_op(), p.get_op(), p.get_op()) , &mut p),
                    "ssetg"   => inst(Inst::SSETG(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "ssetge"  => inst(Inst::SSETGE(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "ssetl"   => inst(Inst::SSETL(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "ssetle"  => inst(Inst::SSETLE(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "brl"     => inst(Inst::BRL(p.get_op(), p.get_op(), p.get_op()),  &mut p),
                    "brg"     => inst(Inst::BRG(p.get_op(), p.get_op(), p.get_op()),  &mut p),
                    "ble"     => inst(Inst::BLE(p.get_op(), p.get_op(), p.get_op()),  &mut p),
                    "brz"     => inst(Inst::BRZ(p.get_op(), p.get_op())            ,  &mut p),
                    "bnz"     => inst(Inst::BNZ(p.get_op(), p.get_op())            ,  &mut p),
                    "setc"    => inst(Inst::SETC(p.get_reg(), p.get_op(), p.get_op()),&mut p),
                    "setnc"   => inst(Inst::SETNC(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "bnc"     => inst(Inst::BNC(p.get_op(), p.get_op(), p.get_op()),  &mut p),
                    "brc"     => inst(Inst::BRC(p.get_op(), p.get_op(), p.get_op()),  &mut p),
                    "sbrl"    => inst(Inst::SBRL(p.get_op(), p.get_op(), p.get_op()), &mut p),
                    "sbrg"    => inst(Inst::SBRG(p.get_op(), p.get_op(), p.get_op()), &mut p),
                    "sble"    => inst(Inst::SBLE(p.get_op(), p.get_op(), p.get_op()), &mut p),
                    "sbge"    => inst(Inst::SBGE(p.get_op(), p.get_op(), p.get_op()), &mut p),
                    "bod"     => inst(Inst::BOD(p.get_op(), p.get_op())             , &mut p),
                    "bev"     => inst(Inst::BEV(p.get_op(), p.get_op())             , &mut p),
                    "brn"     => inst(Inst::BRN(p.get_op(), p.get_op()),              &mut p),
                    "brp"     => inst(Inst::BRP(p.get_op(), p.get_op()),              &mut p),
                    "bsr"     => inst(Inst::BSR(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "bsl"     => inst(Inst::BSL(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "srs"     => inst(Inst::SRS(p.get_reg(), p.get_op())            , &mut p),
                    "bss"     => inst(Inst::BSS(p.get_reg(), p.get_op(), p.get_op()), &mut p),
                    "cal"     => inst(Inst::CAL(p.get_jmp())                        , &mut p),
                    "ret"     => inst(Inst::RET                                     , &mut p),

                    "yomamma" => { p.err.error(&p.buf.current(), ErrorKind::YoMamma); p.buf.advance(); },
                    _ => { p.err.error(&p.buf.current(), ErrorKind::UnknownInstruction); p.buf.advance(); },
                }
                p.ast.debug.pc_to_line_start.push(p.at_line);
            },
            Kind::Label => {
                match p.ast.labels.get(p.buf.current().str) {
                    Some(Label::Defined(_)) => p.err.error(&p.buf.current(), ErrorKind::DuplicatedLabelName),
                    Some(Label::Undefined(v)) => {
                        let label_name = p.buf.current().str;
                        let pc = match p.buf.peek().str.to_lowercase().as_str() {
                            "dw" => p.ast.memory.len(),
                            _ => p.ast.instructions.len()
                        };

                        if dw_lab_repl.get(label_name).is_some() {
                            for i in dw_lab_repl.get(label_name).unwrap().iter() {
                                p.ast.memory[*i as usize] = pc as u64;
                            }
                        }

                        for i in v.references.iter() {
                            p.ast.instructions[*i] = match &p.ast.instructions[*i] {
                                Inst::PSH(a) => Inst::PSH(a.clone().transform_label(label_name, pc)),
                                Inst::JMP(a) => Inst::JMP(a.clone().transform_label(label_name, pc)),
                                Inst::MOV(a, b) => Inst::MOV(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::IN (a, b) => Inst::IN(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::OUT(a, b) => Inst::OUT(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::INC(a, b) => Inst::INC(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::DEC(a, b) => Inst::DEC(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::LSH(a, b) => Inst::LSH(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::RSH(a, b) => Inst::RSH(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::LOD(a, b) => Inst::LOD(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::STR(a, b) => Inst::STR(a.clone().transform_label(label_name, pc), b.clone()),
                                Inst::ADD(a, b, c) => Inst::ADD(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SUB(a, b, c) => Inst::SUB(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::NOR(a, b, c) => Inst::NOR(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BGE(a, b, c) => Inst::BGE(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::NEG(a, b) => Inst::NEG(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::AND(a, b, c) => Inst::AND(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::OR(a, b, c) => Inst::OR(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::NOT(a, b) => Inst::NOT(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::NAND(a, b, c) => Inst::NAND(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::MLT(a, b, c) => Inst::MLT(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::DIV(a, b, c) => Inst::DIV(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::MOD(a, b, c) => Inst::MOD(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::ABS(a, b) => Inst::ABS(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::LLOD(a, b, c) => Inst::LLOD(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::LSTR(a, b, c) => Inst::LSTR(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone()),
                                Inst::SDIV(a, b, c) => Inst::SDIV(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETE(a, b, c) => Inst::SETE(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETNE(a, b, c) => Inst::SETNE(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETG(a, b, c) => Inst::SETG(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETGE(a, b, c) => Inst::SETGE(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETL(a, b, c) => Inst::SETL(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETLE(a, b, c) => Inst::SETLE(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::XOR(a, b, c) => Inst::XOR(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::XNOR(a, b, c) => Inst::XNOR(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BNE(a, b, c) => Inst::BNE(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BRE(a, b, c) => Inst::BRE(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SSETG(a, b, c) => Inst::SSETG(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SSETGE(a, b, c) => Inst::SSETGE(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SSETL(a, b, c) => Inst::SSETL(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SSETLE(a, b, c) => Inst::SSETLE(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BRL(a, b, c) => Inst::BRL(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BRG(a, b, c) => Inst::BRG(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BLE(a, b, c) => Inst::BLE(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BRZ(a, b) => Inst::BRZ(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc)),
                                Inst::BNZ(a, b) => Inst::BNZ(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc)),
                                Inst::SETC(a, b, c) => Inst::SETC(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SETNC(a, b, c) => Inst::SETNC(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BNC(a, b, c) => Inst::BNC(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BRC(a, b, c) => Inst::BRC(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SBRL(a, b, c) => Inst::SBRL(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SBRG(a, b, c) => Inst::SBRG(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SBLE(a, b, c) => Inst::SBLE(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SBGE(a, b, c) => Inst::SBGE(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BOD(a, b) => Inst::BOD(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc)),
                                Inst::BEV(a, b) => Inst::BEV(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc)),
                                Inst::BRN(a, b) => Inst::BRN(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc)),
                                Inst::BRP(a, b) => Inst::BRP(a.clone().transform_label(label_name, pc), b.clone().transform_label(label_name, pc)),
                                Inst::BSR(a, b, c) => Inst::BSR(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::BSL(a, b, c) => Inst::BSL(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::SRS(a, b) => Inst::SRS(a.clone(), b.clone().transform_label(label_name, pc)),
                                Inst::BSS(a, b, c) => Inst::BSS(a.clone(), b.clone().transform_label(label_name, pc), c.clone().transform_label(label_name, pc)),
                                Inst::CAL(a) => Inst::CAL(a.clone().transform_label(label_name, pc)),
                                _ => continue,
                            }
                        }
                        p.ast.labels.insert(p.buf.current().str.to_string(), Label::Defined(p.ast.instructions.len()));
                    },
                    None => {
                        let pc = match p.buf.peek().str.to_lowercase().as_str() {
                            "dw" => p.ast.memory.len(),
                            _ => p.ast.instructions.len()
                        };
                        p.ast.labels.insert(p.buf.current().str.to_string(), Label::Defined(pc));
                    },
                }
                p.buf.advance();
            },
            Kind::Macro => {
                match p.buf.current().str {
                    "@define" => {
                        let to_replace = p.buf.next().str;
                        p.macros.insert(to_replace, p.buf.next());
                    },
                    _ => {p.err.error(&p.buf.current(), ErrorKind::UnexpectedMacro); p.buf.advance()},
                }
            }
            Kind::White | Kind::Comment | Kind::Char | Kind::String => p.buf.advance(),
            Kind::EOF => break,
            Kind::LF => {p.at_line += 1; p.buf.advance()},
            _ => { p.buf.advance(); },
        }
    }

    for (_, el) in p.ast.labels.iter() {
        match el {
            Label::Undefined(a) => {
                for i in a.referenced_tokens.iter() {
                    p.err.error(&p.buf.toks[*i], ErrorKind::UndefinedLabel);
                }
            },
            _ => (),
        }
    }

    let ms = p.ast.memory.len();
    for el in p.ast.instructions.iter_mut() {
        *el = match el {
            Inst::ADD(d, a, b) => Inst::ADD(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::RSH(d, a) => Inst::RSH(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::LOD(d, a) => Inst::LOD(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::STR(d, a) => Inst::STR(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BGE(d, a, b) => Inst::BGE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::NOR(d, a, b) => Inst::NOR(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::MOV(d, a) => Inst::MOV(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::INC(d, a) => Inst::INC(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::DEC(d, a) => Inst::DEC(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::OUT(d, a) => Inst::OUT(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::IN(d, a) => Inst::IN(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),

            Inst::PSH(a) => Inst::PSH(a.clone().transform_mem(ms)),
            Inst::POP(d) => Inst::POP(d.clone().transform_mem(ms)),
            Inst::JMP(d) => Inst::JMP(d.clone().transform_mem(ms)),
            Inst::SUB(d, a, b) => Inst::SUB(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::LSH(d, a) => Inst::LSH(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::NEG(d, a) => Inst::NEG(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::AND(d, a, b) => Inst::AND(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::OR(d, a, b) => Inst::OR(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::NOT(d, a) => Inst::NOT(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::NAND(d, a, b) => Inst::NAND(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::CPY(d, a) => Inst::CPY(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::MLT(d, a, b) => Inst::MLT(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::DIV(d, a, b) => Inst::DIV(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::MOD(d, a, b) => Inst::MOD(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::ABS(d, a) => Inst::ABS(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::LLOD(d, a, b) => Inst::LLOD(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::LSTR(d, a, b) => Inst::LSTR(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SDIV(d, a, b) => Inst::SDIV(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETE(d, a, b) => Inst::SETE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETNE(d, a, b) => Inst::SETNE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETG(d, a, b) => Inst::SETG(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETGE(d, a, b) => Inst::SETGE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETL(d, a, b) => Inst::SETL(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETLE(d, a, b) => Inst::SETLE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::XOR(d, a, b) => Inst::XOR(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::XNOR(d, a, b) => Inst::XNOR(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BNE(d, a, b) => Inst::BNE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BRE(d, a, b) => Inst::BRE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SSETG(d, a, b) => Inst::SSETG(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SSETGE(d, a, b) => Inst::SSETGE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SSETL(d, a, b) => Inst::SSETL(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SSETLE(d, a, b) => Inst::SSETLE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BRL(d, a, b) => Inst::BRL(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BRG(d, a, b) => Inst::BRG(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BLE(d, a, b) => Inst::BLE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BRZ(d, a) => Inst::BRZ(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BNZ(d, a) => Inst::BNZ(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::SETC(d, a, b) => Inst::SETG(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SETNC(d, a, b) => Inst::SETGE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BRC(d, a, b) => Inst::BRC(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BNC(d, a, b) => Inst::BNC(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SBRL(d, a, b) => Inst::SBRL(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SBRG(d, a, b) => Inst::SBRG(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SBLE(d, a, b) => Inst::SBLE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SBGE(d, a, b) => Inst::SBGE(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BOD(d, a) => Inst::BOD(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BEV(d, a) => Inst::BEV(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BRP(d, a) => Inst::BRP(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BRN(d, a) => Inst::BRN(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BSL(d, a, b) => Inst::BSR(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::BSR(d, a, b) => Inst::BSL(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::SRS(d, a) => Inst::SRS(d.clone().transform_mem(ms), a.clone().transform_mem(ms)),
            Inst::BSS(d, a, b) => Inst::BSS(d.clone().transform_mem(ms), a.clone().transform_mem(ms), b.clone().transform_mem(ms)),
            Inst::CAL(d) => Inst::CAL(d.clone().transform_mem(ms)),
            _ => continue
        };
    }

    for i in dw_mem_repl.iter() {
        p.ast.memory[*i as usize] += ms as u64;
    }

    p
}

fn inst<'a>(inst: Inst, p: &mut Parser<'a>) {
    p.ast.instructions.push(inst);
    p.assert_done();
}

impl <'a> Parser<'a> {
    fn parse_dw(&mut self, dw_lab_repl: &mut HashMap<String, Vec<u64>>, dw_mem_repl: &mut Vec<u64>) -> Vec<u64> {
        let a = self.buf.current();
        match a.kind {
            Kind::Int(v) => vec![v as u64],
            Kind::Memory(v) => {
                dw_mem_repl.push(v);
                vec![v]
            },
            Kind::Label => {
                self.buf.advance();
                match self.ast.labels.get(a.str) {
                    Some(Label::Defined(v)) => vec![*v as u64],
                    Some(Label::Undefined(_)) => {
                        dw_lab_repl.get_mut(a.str).unwrap().push(self.ast.memory.len() as u64);
                        vec![0]
                    },
                    _ => {
                        self.ast.labels.insert(a.str.to_string(), Label::Undefined(
                            UndefinedLabel { references: vec![], referenced_tokens: vec![] }
                        ));
                        dw_lab_repl.insert(a.str.to_string(), vec![self.ast.memory.len() as u64]);
                        vec![0]
                    },
                }
            },
            Kind::Macro => {
                let a = self.parse_macro(a.str).unwrap();
                self.buf.advance();
                vec![a]
            },
            Kind::String => {
                let mut text = String::new();
                while self.buf.has_next() {match self.buf.next().kind {
                    Kind::String => break,
                    Kind::Text => text += self.buf.cur().str,
                    Kind::Escape(c) => text.push(c),
                    _ => {
                        self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        break;
                    }
                }}
                text.chars().map(|a| a as u64).collect()
            },
            _ => {
                self.err.error(&a, ErrorKind::YoMamma);
                vec![]
            },
        }
    }
    fn get_reg(&mut self) -> Operand {
        let (ast, op) = self.get_ast_op();
        match ast {
            AstOp::Reg(_) | AstOp::Unknown => {},
            actual => {
                self.err.error(self.buf.cur(), ErrorKind::InvalidOperandType{
                    expected: "register", actual
                });
            }
        }
        op
    }
    fn get_port(&mut self) -> Operand {
        let (ast, op) = self.get_ast_op();
        match ast {
            AstOp::Reg(_) | AstOp::Port(_) | AstOp::Unknown => {},
            actual => {
                self.err.warn(self.buf.cur(), ErrorKind::InvalidOperandType{
                    expected: "port", actual
                });
            }
        }
        op
    }
    fn get_mem(&mut self) -> Operand {
        let (ast, op) = self.get_ast_op();
        match ast {
            AstOp::Reg(_) | AstOp::Mem(_) | AstOp::Unknown => {},
            actual => {
                self.err.warn(self.buf.cur(), ErrorKind::InvalidOperandType{
                    expected: "memory address", actual
                });
            }
        }
        op
    }
    fn get_jmp(&mut self) -> Operand {
        let (ast, op) = self.get_ast_op();
        match ast {
            AstOp::Reg(_) | AstOp::Label(_) | AstOp::JumpLocation(_) | AstOp::Unknown => {},
            actual => {
                self.err.warn(self.buf.cur(), ErrorKind::InvalidOperandType{
                    expected: "jump target", actual
                });
            }
        }
        op
    }
    fn get_imm(&mut self) -> Operand {
        let (ast, op) = self.get_ast_op();
        match ast {
            AstOp::Reg(_) => {
                self.err.warn(self.buf.cur(), ErrorKind::InvalidOperandType{
                    expected: "immediate", actual: ast
                });
            },
            _ => {}
        }
        op
    }

    fn get_op(&mut self) -> Operand {
        self.get_ast_op().1
    }
    fn trans_op(&mut self, op: &AstOp) -> Operand {
        match op {
            AstOp::Unknown => Operand::Imm(0),
            AstOp::Int(v) => Operand::Imm(*v),
            AstOp::Reg(v) => Operand::Reg(*v),
            AstOp::Mem(v) => Operand::Mem(*v),
            AstOp::Port(v) => Operand::Imm(*v),
            AstOp::Char(v) => Operand::Imm(*v as u64),
            AstOp::String(_v) => Operand::Imm(0),
            AstOp::Label(_v) => {
                label_tok_to_operand(&self.buf.current(), self)
            },
            AstOp::JumpLocation(v) => Operand::Imm(*v),
        }
    }
    fn get_ast_op(&mut self) -> (AstOp, Operand){
        self.buf.advance();
        let current = self.buf.current();
        let ast = match current.kind {
            Kind::Reg(v) => AstOp::Reg(v),
            Kind::Int(v) => AstOp::Int(v as u64),
            Kind::Memory(m) => AstOp::Mem(m),
            Kind::PortNum(v) => AstOp::Port(v),
            Kind::Port => {
                match IOPort::from_str(&current.str[1..].to_uppercase()) {
                    Ok(port) => {AstOp::Port(port as u64)},
                    Err(_err) => {
                        self.err.error(&self.buf.current(), ErrorKind::UnknownPort);
                        AstOp::Port(0)
                    }
                }
            }
            Kind::Label  => AstOp::Label(current.str[1..].to_owned()),
            Kind::Char => {
                match self.buf.next().kind {
                    Kind::Text => {
                        let a = self.buf.current();
                        if !matches!(self.buf.next().kind, Kind::Char) {
                            self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        }
                        AstOp::Char(a.str.chars().next().unwrap())
                    }
                    Kind::Escape(c) => {
                        if !matches!(self.buf.next().kind, Kind::Char) {
                            self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        }
                        AstOp::Char(c)
                    }
                    _ => {
                        self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        AstOp::Char('\x00')
                    },
                }
            }
            Kind::String => {
                let mut text = String::new();
                while self.buf.has_next() {match self.buf.next().kind {
                    Kind::String => break,
                    Kind::Text => text += self.buf.cur().str,
                    Kind::Escape(c) => text.push(c),
                    _ => {
                        self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        break;
                    }
                }}


                AstOp::String(text)
            }
            Kind::Relative(v) => {
                AstOp::JumpLocation((self.ast.instructions.len() as i64 + v) as u64)
            }
            Kind::EOF | Kind::LF => {
                self.err.error(&self.buf.current(), ErrorKind::NotEnoughOperands);
                AstOp::Unknown
            }
            Kind::Macro => {
                match self.parse_macro(self.buf.current().str) {
                    Some(v) => AstOp::Int(v),
                    None => AstOp::Unknown
                }
            }
            Kind::Name => {
                self.get_ast_op_from_token(self.macros[current.str].clone()).0
            }
            _ => {
                self.err.error(&self.buf.current(), ErrorKind::InvalidOperand);
                AstOp::Unknown
            }
        };
        let op = self.trans_op(&ast);
        (ast, op)
    }

    fn get_ast_op_from_token(&mut self, current: UToken<'a>) -> (AstOp, Operand) {
        let ast = match current.kind {
            Kind::Reg(v) => AstOp::Reg(v),
            Kind::Int(v) => AstOp::Int(v as u64),
            Kind::Memory(m) => AstOp::Mem(m),
            Kind::PortNum(v) => AstOp::Port(v),
            Kind::Port => {
                match IOPort::from_str(&current.str[1..].to_uppercase()) {
                    Ok(port) => {AstOp::Port(port as u64)},
                    Err(_err) => {
                        self.err.error(&self.buf.current(), ErrorKind::UnknownPort);
                        AstOp::Port(0)
                    }
                }
            }
            Kind::Label  => AstOp::Label(current.str[1..].to_owned()),
            Kind::Char => {
                match self.buf.next().kind {
                    Kind::Text => {
                        let a = self.buf.current();
                        if !matches!(self.buf.next().kind, Kind::Char) {
                            self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        }
                        AstOp::Char(a.str.chars().next().unwrap())
                    }
                    Kind::Escape(c) => {
                        if !matches!(self.buf.next().kind, Kind::Char) {
                            self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        }
                        AstOp::Char(c)
                    }
                    _ => {
                        self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        AstOp::Char('\x00')
                    },
                }
            }
            Kind::String => {
                let mut text = String::new();
                while self.buf.has_next() {match self.buf.next().kind {
                    Kind::String => break,
                    Kind::Text => text += self.buf.cur().str,
                    Kind::Escape(c) => text.push(c),
                    _ => {
                        self.err.error(&self.buf.current(), ErrorKind::EOFBeforeEndOfString);
                        break;
                    }
                }}


                AstOp::String(text)
            }
            Kind::Relative(v) => {
                AstOp::JumpLocation((self.ast.instructions.len() as i64 + v) as u64)
            }
            Kind::EOF | Kind::LF => {
                self.err.error(&self.buf.current(), ErrorKind::NotEnoughOperands);
                AstOp::Unknown
            }
            Kind::Macro => {
                match self.parse_macro(current.str) {
                    Some(v) => AstOp::Int(v),
                    None => AstOp::Unknown
                }
            }
            Kind::Name => {
                self.get_ast_op_from_token(self.macros[current.str].clone()).0
            }
            _ => {
                self.err.error(&self.buf.current(), ErrorKind::InvalidOperand);
                AstOp::Unknown
            }
        };
        let op = self.trans_op(&ast);
        (ast, op)
    }

    fn parse_macro(&self, m: &str) -> Option<u64> {
        match m.to_lowercase().as_str() {
            "@max" => Some(u64::MAX),
            "@msb" => Some(1 << 63),
            "@smax" => Some(i64::MAX as u64),
            "@bits" => Some(self.ast.headers.bits),
            "@minheap" => Some(self.ast.headers.minheap),
            _ => None
        }
    }

    fn assert_done(&mut self) {
        self.buf.advance();
        match self.buf.current().kind {
            Kind::LF |  Kind::EOF => {},
            _ => {
                self.err.error(&self.buf.current(), ErrorKind::ToManyOperands);
                while match self.buf.current().kind {Kind::LF |  Kind::EOF => false, _ => true} {
                    self.buf.advance()
                } 
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct UndefinedLabel {
    references: Vec<usize>,
    referenced_tokens: Vec<usize>,
}

#[derive(Debug, PartialEq)]
pub enum Label {
    Undefined(UndefinedLabel),
    Defined(usize),
}

fn label_tok_to_operand<'a>(tok: &UToken<'a>, p: &mut Parser) -> Operand {
    if (*tok).kind != Kind::Label {return Operand::Imm(0);}

    match p.ast.labels.get(tok.str) {
        Some(Label::Undefined(v)) => {
            let mut a = v.clone();
            a.references         .push(p.ast.instructions.len());
            a.referenced_tokens  .push(p.buf.index);
            p.ast.labels.insert((*tok).str.to_string(), Label::Undefined(a));
            Operand::Label(tok.str.to_string())
        },
        Some(Label::Defined(v)) => Operand::Imm(*v as u64),
        None => {
            p.ast.labels.insert((*tok).str.to_string(), Label::Undefined(
                UndefinedLabel{
                    references: vec![p.ast.instructions.len()],
                    referenced_tokens: vec![p.buf.index]
                }
            ));
            Operand::Label(tok.str.to_string())
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub headers: Headers,
    pub instructions: Vec<Inst>,
    pub labels: HashMap<String, Label>,
    pub memory: Vec<u64>,
    pub debug: DebugInfo,
}

impl Program {
    pub fn new(src: Rc<str>) -> Self {
        Self { headers: Headers::new(), instructions: Vec::new(), labels: HashMap::new(), memory: Vec::new(), debug: DebugInfo::new(src) }
    }
}

#[derive(Debug)]
pub struct DebugInfo {
    pub src: Rc<str>,
    pub pc_to_line_start: Vec<usize>
}
impl DebugInfo {
    pub fn new(src: Rc<str>) -> Self {
        Self {src, pc_to_line_start: Vec::new()}
    }
}

#[derive(Debug, Clone)] // cant copy because of the String
pub enum AstOp {
    Unknown,
    Int(u64),
    Reg(u64),
    Mem(u64),
    Port(u64),
    Char(char),
    String(String),
    Label(String),
    JumpLocation(u64),
}

#[derive(Debug, Clone)] // cant copy because of the String
pub enum Operand {
    Imm(u64),
    Mem(u64), // should be compiled into Imm before emulating
    Reg(u64),
    Label(String),
}

impl Operand {
    pub fn transform_label(self, label: &str, pc: usize) -> Self {
        if matches!(self, Self::Label(ref l) if l == label) {
            Self::Imm(pc as u64)
        } else {
            self
        }
    }

    pub fn transform_mem(self, ms: usize) -> Self {
        match self {
            Self::Mem(v) => Self::Imm(v + ms as u64),
            _ => self
        }
    }
}

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
    NEG(Operand, Operand),
    AND(Operand, Operand, Operand),
    OR(Operand, Operand, Operand),
    NOT(Operand, Operand),
    NAND(Operand, Operand, Operand),
    CPY(Operand, Operand),
    MLT(Operand, Operand, Operand),
    DIV(Operand, Operand, Operand),
    MOD(Operand, Operand, Operand),
    ABS(Operand, Operand),
    LLOD(Operand, Operand, Operand),
    LSTR(Operand, Operand, Operand),
    SDIV(Operand, Operand, Operand),
    SETE(Operand, Operand, Operand),
    SETNE(Operand, Operand, Operand),
    SETG(Operand, Operand, Operand),
    SETGE(Operand, Operand, Operand),
    SETL(Operand, Operand, Operand),
    SETLE(Operand, Operand, Operand),
    XOR(Operand, Operand, Operand),
    XNOR(Operand, Operand, Operand),
    BNE(Operand, Operand, Operand),
    BRE(Operand, Operand, Operand),
    SSETG(Operand, Operand, Operand),
    SSETGE(Operand, Operand, Operand),
    SSETL(Operand, Operand, Operand),
    SSETLE(Operand, Operand, Operand),
    BRL(Operand, Operand, Operand),
    BRG(Operand, Operand, Operand),
    BLE(Operand, Operand, Operand),
    BRZ(Operand, Operand),
    BNZ(Operand, Operand),
    SETC(Operand, Operand, Operand),
    SETNC(Operand, Operand, Operand),
    BNC(Operand, Operand, Operand),
    BRC(Operand, Operand, Operand),
    SBRL(Operand, Operand, Operand),
    SBRG(Operand, Operand, Operand),
    SBLE(Operand, Operand, Operand),
    SBGE(Operand, Operand, Operand),
    BOD(Operand, Operand),
    BEV(Operand, Operand),
    BRN(Operand, Operand),
    BRP(Operand, Operand),
    BSR(Operand, Operand, Operand),
    BSL(Operand, Operand, Operand),
    SRS(Operand, Operand),
    BSS(Operand, Operand, Operand),
    CAL(Operand),
    RET,
}
