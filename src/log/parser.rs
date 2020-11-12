use std::result::Result as RResult;
use std::str::FromStr;

use futures::AsyncBufReadExt;
use futures::Stream;
use futures::StreamExt;
use futures::TryStreamExt;
use pom::parser::Parser as PomParser;
use shiplift::tty::TtyChunk;

use crate::log::LogItem;
use crate::log::util::*;

type IoResult<T> = RResult<T, futures::io::Error>;

pub fn buffer_stream_to_line_stream<S>(stream: S) -> impl Stream<Item = IoResult<String>>
    where S: Stream<Item = shiplift::Result<TtyChunk>> + std::marker::Unpin
{
    stream.map(|r| r.map(TtyChunkBuf::from))
        .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
        .into_async_read()
        .lines()
}

pub fn parser<'a>() -> PomParser<'a, u8, LogItem> {
    use pom::parser::*;

    let number = one_of(b"0123456789")
        .repeat(1..)
        .collect()
        .convert(|b| String::from_utf8(b.to_vec()))
        .convert(|s| usize::from_str(&s));

    fn ignored<'a>() -> PomParser<'a, u8, Vec<u8>> {
        none_of(b"\n").repeat(0..)
    }

    fn string<'a>() -> PomParser<'a, u8, String> {
        let special_char = sym(b'\\') | sym(b'/') | sym(b'"')
                | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
                | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
        let escape_sequence = sym(b'\\') * special_char;
        let string = (none_of(b"\\\"") | escape_sequence).repeat(0..);

        string.convert(String::from_utf8)
    }

    (
        seq(b"#BUTIDO:") * (
            (seq(b"PROGRESS:") * number.map(|n| LogItem::Progress(n)))
            |
            (seq(b"PHASE:") * string().map(|s| LogItem::CurrentPhase(s)))
            |
            (
                (seq(b"STATE:ERR:") * string().map(|s| LogItem::State(Err(s))))
                |
                (seq(b"STATE:OK:") * string().map(|s| LogItem::State(Ok(s))))
            )
        )
    )
    | ignored().map(|s| LogItem::Line(Vec::from(s)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use anyhow::Result;

    // Helper function for showing log item in error message in pretty
    fn prettify_item(e: &LogItem) -> String {
        match e {
            LogItem::Line(buf) => {
                let line = String::from_utf8(buf.to_vec()).unwrap();
                format!("LogItem::Line({})", line)
            },
            other => format!("{:?}", other),
        }
    }


    #[test]
    fn test_non_log() {
        let s = "foo bar";
        let p = parser();
        let r = p.parse(s.as_bytes());

        assert!(r.is_ok(), "Not ok: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r, LogItem::Line("foo bar".bytes().collect()));
    }

    #[test]
    fn test_progress_1() {
        let s = "#BUTIDO:PROGRESS:1";
        let p = parser();
        let r = p.parse(s.as_bytes());

        assert!(r.is_ok(), "Not ok: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r, LogItem::Progress(1));
    }

    #[test]
    fn test_progress_100() {
        let s = "#BUTIDO:PROGRESS:100";
        let p = parser();
        let r = p.parse(s.as_bytes());

        assert!(r.is_ok(), "Not ok: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r, LogItem::Progress(100));
    }

    #[test]
    fn test_progress_negative() {
        let s = "#BUTIDO:PROGRESS:-1";
        let p = parser();
        let r = p.parse(s.as_bytes());

        assert!(r.is_ok(), "Not ok: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r, LogItem::Line("#BUTIDO:PROGRESS:-1".bytes().collect()));
    }

    #[test]
    fn test_phase() {
        let s = "#BUTIDO:PHASE:a";
        let p = parser();
        let r = p.parse(s.as_bytes());

        assert!(r.is_ok(), "Not ok: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r, LogItem::CurrentPhase(String::from("a")), "Expected CurrentPhase(a), got: {}", prettify_item(&r));
    }

    #[test]
    fn test_phase_multiline() {
        let s = "#BUTIDO:PHASE:a

            ";
        let p = parser();
        let r = p.parse(s.as_bytes());

        assert!(r.is_ok(), "Not ok: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r, LogItem::CurrentPhase(String::from("a\n\n            ")), "Expected CurrentPhase(a), got: {}", prettify_item(&r));
    }

    #[test]
    fn test_multiline() {
        let buffer: &'static str = indoc::indoc! {"
            #BUTIDO:PROGRESS:0
            Some log line
            #BUTIDO:PHASE:configure
            Some log line
            Some log line
            Some log line
            #BUTIDO:PHASE:Build
            Some other log line
            Some other log line
            Some other log line
            #BUTIDO:STATE:OK:finished successfully
        "};

        let p = parser();

        let res = buffer
            .lines()
            .map(|line| p.parse(line.as_bytes()).map_err(Error::from))
            .collect::<Result<Vec<_>>>();

        assert!(res.is_ok());
        let res   = res.unwrap();
        let mut i = res.iter();

        {
            let elem = i.next().unwrap();
            let expe = LogItem::Progress(0);
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            let elem = i.next().unwrap();
            let expe = LogItem::Line("Some log line".bytes().collect());
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            let elem = i.next().unwrap();
            let expe = LogItem::CurrentPhase(String::from("configure"));
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            let expe = LogItem::Line("Some log line".bytes().collect());

            let elem = i.next().unwrap();
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));

            let elem = i.next().unwrap();
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));

            let elem = i.next().unwrap();
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            let elem = i.next().unwrap();
            let expe = LogItem::CurrentPhase(String::from("Build"));
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            let expe = LogItem::Line("Some other log line".bytes().collect());

            let elem = i.next().unwrap();
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));

            let elem = i.next().unwrap();
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));

            let elem = i.next().unwrap();
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            let elem = i.next().unwrap();
            let expe = LogItem::State(Ok(String::from("finished successfully")));
            assert_eq!(*elem, expe, "Expected {}: {:?}", prettify_item(&expe), prettify_item(elem));
        }
        {
            assert!(i.next().is_none());
        }
    }
}

