use std::num::TryFromIntError;

use async_recursion::async_recursion;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

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

async fn read_crlf(stream: &mut TcpStream) -> Result<(), ParseError> {
    let mut buf = [0; 2];

    stream.read_exact(&mut buf).await?;

    if buf[0] == b'\r' && buf[1] == b'\n' {
        Ok(())
    } else {
        Err(ParseError::MissingCRLF)
    }
}

async fn read_until_crlf(stream: &mut TcpStream) -> Result<String, ParseError> {
    let mut write = Vec::new();
    let mut buf = [0; 1];

    loop {
        if stream.read(&mut buf).await? == 0 {
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

async fn read_i64(stream: &mut TcpStream) -> Result<i64, ParseError> {
    Ok(read_until_crlf(stream).await?.parse::<i64>()?)
}

#[async_recursion]
pub async fn parse(
    stream: &mut TcpStream,
    allow_pipeline: bool,
) -> Result<Option<Data>, ParseError> {
    let mut buf = [0];

    let n = stream.read(&mut buf).await?;

    if n == 0 {
        return Ok(None);
    }

    Ok(match buf[0] {
        b'+' => Some(parse_string(stream).await?),
        b'-' => Some(parse_error(stream).await?),
        b':' => Some(parse_integer(stream).await?),
        b'*' => Some(parse_array(stream).await?),
        b'$' => Some(parse_bulk_string(stream).await?),
        _ if allow_pipeline => Some(parse_pipeline(stream, buf[0]).await?),
        _ => None,
    })
}

async fn parse_string(stream: &mut TcpStream) -> Result<Data, ParseError> {
    Ok(Data::String(read_until_crlf(stream).await?))
}

async fn parse_error(stream: &mut TcpStream) -> Result<Data, ParseError> {
    Ok(Data::Error(read_until_crlf(stream).await?))
}

async fn parse_integer(stream: &mut TcpStream) -> Result<Data, ParseError> {
    Ok(Data::Integer(
        read_until_crlf(stream).await?.parse::<i64>()?,
    ))
}

#[async_recursion]
async fn parse_array(stream: &mut TcpStream) -> Result<Data, ParseError> {
    let length = read_i64(stream).await?;

    if length == -1 {
        return Ok(Data::NullArray);
    }

    let length = length.try_into()?;

    let mut results = Vec::with_capacity(length);

    while results.len() < length {
        if let Some(item) = parse(stream, false).await? {
            results.push(item);
        } else {
            break;
        }
    }

    Ok(Data::Array(results))
}

async fn parse_bulk_string(stream: &mut TcpStream) -> Result<Data, ParseError> {
    let length = read_i64(stream).await?;

    if length == -1 {
        return Ok(Data::NullBulkString);
    }

    let mut buf = vec![0; length.try_into()?];

    stream.read_exact(&mut buf).await?;
    read_crlf(stream).await?;

    Ok(Data::BulkString(String::from_utf8(buf)?))
}

async fn parse_pipeline(stream: &mut TcpStream, first: u8) -> Result<Data, ParseError> {
    let mut string = (first as char).to_string();

    string.extend(read_until_crlf(stream).await);

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
