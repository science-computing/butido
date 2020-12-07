use pom::parser::*;
use pom::parser::Parser as PomParser;

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

pub fn equal<'a>() -> PomParser<'a, u8, Vec<u8>> {
    sym(b'=').map(|b| vec![b])
}

pub fn nonempty_string_with_optional_quotes<'a>() -> Parser<'a, u8, String> {
    let special_char = ||
        sym(b'\\') | sym(b'/') | sym(b'"')
        | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
        | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
    let escape_sequence = || sym(b'\\') * special_char();

    let inner_string = || (none_of(b"\\\"") | escape_sequence()).repeat(1..);

    let string = (sym(b'"') * inner_string() - sym(b'"')) | inner_string();
    string.convert(String::from_utf8)
}

