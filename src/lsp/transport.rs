extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::str;
use std::io::{self, BufRead, Write};

pub fn read_framed_message<R: BufRead>(reader: &mut R) -> io::Result<String> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF while reading LSP headers",
            ));
        }

        if line == "\r\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        let Some((name, value)) = trimmed.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("malformed header line: {trimmed}"),
            ));
        };

        if name.eq_ignore_ascii_case("Content-Length") {
            let parsed = value.trim().parse::<usize>().map_err(|_parse_error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length value: {}", value.trim()),
                )
            })?;
            content_length = Some(parsed);
        }
    }

    let len = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;

    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload)?;
    let text = str::from_utf8(&payload).map_err(|_utf8_error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "message payload is not valid UTF-8",
        )
    })?;
    Ok(text.to_owned())
}

pub fn write_framed_message<W: Write>(writer: &mut W, payload: &str) -> io::Result<()> {
    let bytes = payload.as_bytes();
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(bytes)?;
    writer.flush()?;
    Ok(())
}
