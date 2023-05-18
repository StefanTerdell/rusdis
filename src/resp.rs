use std::{num::TryFromIntError, slice::Iter};

#[derive(Debug)]
pub enum Data {
    String(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Array(Vec<Data>),
    NullBulkString,
    NullArray,
}

pub fn ser(data: Data) -> Vec<u8> {
    match data {
        Data::String(str) => format!("+{}\r\n", str).into_bytes(),
        Data::Error(str) => format!("-{}\r\n", str).into_bytes(),
        Data::Integer(int) => format!(":{}\r\n", int).into_bytes(),
        Data::BulkString(str) => format!("${}\r\n{}\r\n", str.len(), str).into_bytes(),
        Data::Array(arr) => {
            let mut output = format!("*{}\r\n", arr.len()).into_bytes();
            for element in arr {
                output.extend(ser(element));
            }
            output
        }
        Data::NullBulkString => b"$-1\r\n".to_vec(),
        Data::NullArray => b"*-1\r\n".to_vec(),
    }
}

pub fn ser_string(str: &str) -> Vec<u8> {
    ser(Data::String(str.to_string()))
}

pub fn ser_error(str: &str) -> Vec<u8> {
    ser(Data::Error(str.to_string()))
}

pub fn ser_int(int: i64) -> Vec<u8> {
    ser(Data::Integer(int))
}

pub fn ser_bulk_string(str: &str) -> Vec<u8> {
    ser(Data::BulkString(str.to_string()))
}

pub fn ser_null_bulk_string() -> Vec<u8> {
    ser(Data::NullBulkString)
}

// pub fn ser_array(arr: Vec<Data>) -> Vec<u8> {
//     ser(Data::Array(arr))
// }

// pub fn ser_null_array() -> Vec<u8> {
//     ser(Data::NullArray)
// }

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Int(std::num::ParseIntError),
    Utf8(std::string::FromUtf8Error),
    NegativeInt,
    MissingCRLF,
    UnexpectedEnding,
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> ParseError {
        ParseError::Io(err)
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(err: std::num::ParseIntError) -> ParseError {
        ParseError::Int(err)
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(err: std::string::FromUtf8Error) -> ParseError {
        ParseError::Utf8(err)
    }
}

impl From<TryFromIntError> for ParseError {
    fn from(_: TryFromIntError) -> ParseError {
        ParseError::NegativeInt
    }
}

fn read_crlf(read_buf: &mut Iter<u8>) -> Result<(), ParseError> {
    if let Ok(x) = read_exact(read_buf, 2) {
        if x == "\r\n" {
            return Ok(());
        }
    }

    Err(ParseError::MissingCRLF)
}

fn read_exact(read_buf: &mut Iter<u8>, length: usize) -> Result<String, ParseError> {
    let mut write_buf = Vec::with_capacity(length);

    while let Some(x) = read_buf.next() {
        write_buf.push(*x);

        if write_buf.len() == length {
            return Ok(String::from_utf8(write_buf)?);
        }
    }

    Err(ParseError::UnexpectedEnding)
}

fn read_until_crlf(read_buf: &mut Iter<u8>) -> Result<String, ParseError> {
    let mut write_buf = Vec::new();
    let mut last = [0, 0];

    while let Some(x) = read_buf.next() {
        last[0] = last[1];
        last[1] = *x;

        if last[0] == b'\r' && last[1] == b'\n' {
            write_buf.truncate(write_buf.len() - 1);
            return Ok(String::from_utf8(write_buf)?);
        }

        write_buf.push(*x);
    }

    Err(ParseError::MissingCRLF)
}

fn read_i64(read_buf: &mut Iter<u8>) -> Result<i64, ParseError> {
    Ok(read_until_crlf(read_buf)?.parse::<i64>()?)
}

pub fn parse(read_buf: &mut Iter<u8>, allow_pipeline: bool) -> Result<Option<Data>, ParseError> {
    if let Some(x) = read_buf.next() {
        Ok(match x {
            b'+' => Some(parse_string(read_buf)?),
            b'-' => Some(parse_error(read_buf)?),
            b':' => Some(parse_integer(read_buf)?),
            b'*' => Some(parse_array(read_buf)?),
            b'$' => Some(parse_bulk_string(read_buf)?),
            _ if allow_pipeline => Some(parse_pipeline(read_buf, *x)?),
            _ => None,
        })
    } else {
        Ok(None)
    }
}

fn parse_string(read_buf: &mut Iter<u8>) -> Result<Data, ParseError> {
    Ok(Data::String(read_until_crlf(read_buf)?))
}

fn parse_error(read_buf: &mut Iter<u8>) -> Result<Data, ParseError> {
    Ok(Data::Error(read_until_crlf(read_buf)?))
}

fn parse_integer(read_buf: &mut Iter<u8>) -> Result<Data, ParseError> {
    Ok(Data::Integer(read_until_crlf(read_buf)?.parse::<i64>()?))
}

fn parse_array(read_buf: &mut Iter<u8>) -> Result<Data, ParseError> {
    let length = read_i64(read_buf)?;

    if length == -1 {
        return Ok(Data::NullArray);
    }

    let length = length.try_into()?;

    let mut results = Vec::with_capacity(length);

    while let Ok(Some(item)) = parse(read_buf, false) {
        results.push(item);

        if results.len() == length {
            return Ok(Data::Array(results));
        }
    }

    Err(ParseError::UnexpectedEnding)
}

fn parse_bulk_string(read_buf: &mut Iter<u8>) -> Result<Data, ParseError> {
    let length = read_i64(read_buf)?;

    if length == -1 {
        return Ok(Data::NullBulkString);
    }

    let content = read_exact(read_buf, length.try_into()?)?;

    read_crlf(read_buf)?;

    Ok(Data::BulkString(content))
}

fn parse_pipeline(read_buf: &mut Iter<u8>, first: u8) -> Result<Data, ParseError> {
    let mut content = (first as char).to_string();

    content.extend(read_until_crlf(read_buf));

    Ok(Data::Array(
        content
            .split(' ')
            .filter(|slice| !slice.is_empty())
            .map(|slice| match slice.parse::<i64>() {
                Ok(int) => Data::Integer(int),
                Err(_) => Data::String(slice.to_string()),
            })
            .collect(),
    ))
}
