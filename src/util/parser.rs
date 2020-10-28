use pom::*;
use pom::parser::Parser as PomParser;
use pom::parser::*;
use pom::char_class::hex_digit;

pub fn numbers<'a>() -> PomParser<'a, u8, Vec<u8>> {
    one_of(b"0123456789").repeat(1..)
}

pub fn letters<'a>() -> PomParser<'a, u8, Vec<u8>> {
    pom::parser::is_a(pom::char_class::alpha).repeat(1..)
}

pub fn dash<'a>() -> PomParser<'a, u8, Vec<u8>> {
    sym(b'-').map(|b| vec![b])
}

pub fn under<'a>() -> PomParser<'a, u8, Vec<u8>> {
    sym(b'_').map(|b| vec![b])
}

pub fn dot<'a>() -> PomParser<'a, u8, Vec<u8>> {
    sym(b'.').map(|b| vec![b])
}
