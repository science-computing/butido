//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use bollard::container::LogOutput;
use bytes::Bytes;

#[allow(clippy::enum_variant_names)]
pub enum TtyChunkBuf {
    StdIn(Bytes),
    StdOut(Bytes),
    StdErr(Bytes),
    Console(Bytes),
}

impl From<LogOutput> for TtyChunkBuf {
    fn from(c: LogOutput) -> Self {
        match c {
            LogOutput::StdIn { message } => TtyChunkBuf::StdIn(message),
            LogOutput::StdOut { message } => TtyChunkBuf::StdOut(message),
            LogOutput::StdErr { message } => TtyChunkBuf::StdErr(message),
            LogOutput::Console { message } => TtyChunkBuf::Console(message),
        }
    }
}

impl AsRef<[u8]> for TtyChunkBuf {
    fn as_ref(&self) -> &[u8] {
        match self {
            TtyChunkBuf::StdIn(buffer) => buffer.as_ref(),
            TtyChunkBuf::StdOut(buffer) => buffer.as_ref(),
            TtyChunkBuf::StdErr(buffer) => buffer.as_ref(),
            TtyChunkBuf::Console(buffer) => buffer.as_ref(),
        }
    }
}
