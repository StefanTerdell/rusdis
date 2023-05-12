use std::{
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    num::TryFromIntError,
};

#[derive(Debug)]
pub enum RespData {
    String(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Array(Vec<RespData>),
    NullBulkString,
    NullArray,
}

#[derive(Debug)]
pub enum RespError {
    Io(std::io::Error),
    Int(std::num::ParseIntError),
    Utf8(std::string::FromUtf8Error),
    NegativeInt,
    MissingCRLF,
}

impl From<std::io::Error> for RespError {
    fn from(err: std::io::Error) -> RespError {
        RespError::Io(err)
    }
}

impl From<std::num::ParseIntError> for RespError {
    fn from(err: std::num::ParseIntError) -> RespError {
        RespError::Int(err)
    }
}

impl From<std::string::FromUtf8Error> for RespError {
    fn from(err: std::string::FromUtf8Error) -> RespError {
        RespError::Utf8(err)
    }
}

impl From<TryFromIntError> for RespError {
    fn from(_: TryFromIntError) -> RespError {
        RespError::NegativeInt
    }
}

trait ReaderExtensions {
    fn read_crlf(&mut self) -> Result<(), RespError>;
    fn read_until_crlf(&mut self) -> Result<String, RespError>;
    fn read_i64(&mut self) -> Result<i64, RespError>;
}

impl ReaderExtensions for &mut BufReader<&TcpStream> {
    fn read_crlf(&mut self) -> Result<(), RespError> {
        let mut buf = [0; 2];

        self.read_exact(&mut buf)?;

        if buf[0] == b'\r' && buf[1] == b'\n' {
            Ok(())
        } else {
            Err(RespError::MissingCRLF)
        }
    }

    fn read_until_crlf(&mut self) -> Result<String, RespError> {
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

    fn read_i64(&mut self) -> Result<i64, RespError> {
        Ok(self.read_until_crlf()?.parse::<i64>()?)
    }
}

pub fn parse_resp(
    reader: &mut BufReader<&TcpStream>,
    allow_pipeline: bool,
) -> Result<Option<RespData>, RespError> {
    let mut buf = [0];

    let n = reader.read(&mut buf)?;

    if n == 0 {
        return Ok(None);
    }

    Ok(match buf[0] {
        b'+' => Some(parse_resp_string(reader)?),
        b'-' => Some(parse_resp_error(reader)?),
        b':' => Some(parse_resp_integer(reader)?),
        b'*' => Some(parse_resp_array(reader)?),
        b'$' => Some(parse_resp_bulk_string(reader)?),
        _ if allow_pipeline => Some(parse_pipeline(reader, buf[0])?),
        _ => None,
    })
}

fn parse_resp_string(mut reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok(RespData::String(reader.read_until_crlf()?))
}

fn parse_resp_error(mut reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok(RespData::Error(reader.read_until_crlf()?))
}

fn parse_resp_integer(mut reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok(RespData::Integer(reader.read_until_crlf()?.parse::<i64>()?))
}

fn parse_resp_array(mut reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    let length = reader.read_i64()?;

    if length == -1 {
        return Ok(RespData::NullArray);
    }

    let length = length.try_into()?;

    let mut results = Vec::with_capacity(length);

    while results.len() < length {
        if let Some(item) = parse_resp(reader, false)? {
            results.push(item);
        } else {
            break;
        }
    }

    Ok(RespData::Array(results))
}

fn parse_resp_bulk_string(mut reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    let length = reader.read_i64()?;

    if length == -1 {
        return Ok(RespData::NullBulkString);
    }

    let mut buf = vec![0; length.try_into()?];

    reader.read_exact(&mut buf)?;
    reader.read_crlf()?;

    Ok(RespData::BulkString(String::from_utf8(buf)?))
}

fn parse_pipeline(reader: &mut BufReader<&TcpStream>, first: u8) -> Result<RespData, RespError> {
    let mut string = (first as char).to_string();

    reader.read_line(&mut string)?;

    Ok(RespData::Array(
        string
            .split(' ')
            .filter(|slice| !slice.is_empty())
            .map(|slice| match slice.parse::<i64>() {
                Ok(int) => RespData::Integer(int),
                Err(_) => RespData::String(slice.to_string()),
            })
            .collect(),
    ))
}
