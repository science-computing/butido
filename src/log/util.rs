use shiplift::tty::TtyChunk;

pub enum TtyChunkBuf {
    StdIn (Vec<u8>),
    StdOut(Vec<u8>),
    StdErr(Vec<u8>),
}

impl From<TtyChunk> for TtyChunkBuf {
    fn from(c: TtyChunk) -> Self {
        match c {
            TtyChunk::StdIn(buffer)  => TtyChunkBuf::StdIn (buffer),
            TtyChunk::StdOut(buffer) => TtyChunkBuf::StdOut(buffer),
            TtyChunk::StdErr(buffer) => TtyChunkBuf::StdErr(buffer),
        }
    }
}

impl AsRef<[u8]> for TtyChunkBuf {
    fn as_ref(&self) -> &[u8] {
        match self {
            TtyChunkBuf::StdIn(buffer)  => buffer.as_ref(),
            TtyChunkBuf::StdOut(buffer) => buffer.as_ref(),
            TtyChunkBuf::StdErr(buffer) => buffer.as_ref(),
        }
    }
}

