use std::{io::Read, net::TcpStream, num::TryFromIntError};

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

trait ReaderExtensions {
    fn read_crlf(&mut self) -> Result<(), ParseError>;
    fn read_until_crlf(&mut self) -> Result<String, ParseError>;
    fn read_i64(&mut self) -> Result<i64, ParseError>;
}

impl ReaderExtensions for &mut TcpStream {
    fn read_crlf(&mut self) -> Result<(), ParseError> {
        let mut buf = [0; 2];

        self.read_exact(&mut buf)?;

        if buf[0] == b'\r' && buf[1] == b'\n' {
            Ok(())
        } else {
            Err(ParseError::MissingCRLF)
        }
    }

    fn read_until_crlf(&mut self) -> Result<String, ParseError> {
        let mut write = Vec::new();
        let mut buf = [0; 1];

        loop {
            if self.read(&mut buf)? == 0 {
                break;
            }

            write.push(buf[0]);

            let len = write.len();

            if len > 1 && write[len - 2] == b'\r' && write[len - 1] == b'\n' {
                write.truncate(len - 2);
                break;
            }
        }

        let string = String::from_utf8(write)?;

        Ok(string)
    }

    fn read_i64(&mut self) -> Result<i64, ParseError> {
        Ok(self.read_until_crlf()?.parse::<i64>()?)
    }
}

pub fn parse(
    reader: &mut TcpStream,
    allow_pipeline: bool,
) -> Result<Option<Data>, ParseError> {
    let mut buf = [0];

    let n = reader.read(&mut buf)?;

    if n == 0 {
        return Ok(None);
    }

    Ok(match buf[0] {
        b'+' => Some(parse_string(reader)?),
        b'-' => Some(parse_error(reader)?),
        b':' => Some(parse_integer(reader)?),
        b'*' => Some(parse_array(reader)?),
        b'$' => Some(parse_bulk_string(reader)?),
        _ if allow_pipeline => Some(parse_pipeline(reader, buf[0])?),
        _ => None,
    })
}

fn parse_string(mut reader: &mut TcpStream) -> Result<Data, ParseError> {
    Ok(Data::String(reader.read_until_crlf()?))
}

fn parse_error(mut reader: &mut TcpStream) -> Result<Data, ParseError> {
    Ok(Data::Error(reader.read_until_crlf()?))
}

fn parse_integer(mut reader: &mut TcpStream) -> Result<Data, ParseError> {
    Ok(Data::Integer(reader.read_until_crlf()?.parse::<i64>()?))
}

fn parse_array(mut reader: &mut TcpStream) -> Result<Data, ParseError> {
    let length = reader.read_i64()?;

    if length == -1 {
        return Ok(Data::NullArray);
    }

    let length = length.try_into()?;

    let mut results = Vec::with_capacity(length);

    while results.len() < length {
        if let Some(item) = parse(reader, false)? {
            results.push(item);
        } else {
            break;
        }
    }

    Ok(Data::Array(results))
}

fn parse_bulk_string(mut reader: &mut TcpStream) -> Result<Data, ParseError> {
    let length = reader.read_i64()?;

    if length == -1 {
        return Ok(Data::NullBulkString);
    }

    let mut buf = vec![0; length.try_into()?];

    reader.read_exact(&mut buf)?;
    reader.read_crlf()?;

    Ok(Data::BulkString(String::from_utf8(buf)?))
}

fn parse_pipeline(mut reader: &mut TcpStream, first: u8) -> Result<Data, ParseError> {
    let mut string = (first as char).to_string();

    string.extend(reader.read_until_crlf());

    Ok(Data::Array(
        string
            .split(' ')
            .filter(|slice| !slice.is_empty())
            .map(|slice| match slice.parse::<i64>() {
                Ok(int) => Data::Integer(int),
                Err(_) => Data::String(slice.to_string()),
            })
            .collect(),
    ))
}
