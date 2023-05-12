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
    NilBulkString,
    Array(Vec<RespData>),
    NilArray,
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

pub fn parse_resp(reader: &mut BufReader<&TcpStream>) -> Result<Option<RespData>, RespError> {
    let mut buf = [0];

    let n = reader.read(&mut buf)?;

    if n == 0 {
        return Ok(None);
    }

    Ok(Some(match buf[0] {
        b'+' => parse_resp_string(reader)?,
        b'-' => parse_resp_error(reader)?,
        b':' => parse_resp_integer(reader)?,
        b'*' => parse_resp_array(reader)?,
        b'$' => parse_resp_bulk_string(reader)?,
        _ => parse_resp_inline_command(reader, buf[0])?,
    }))
}

fn parse_resp_string(reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok(RespData::String(read_string(reader)?))
}

fn parse_resp_error(reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok(RespData::Error(read_string(reader)?))
}

fn parse_resp_integer(reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok(RespData::Integer(read_string(reader)?.parse::<i64>()?))
}

fn parse_resp_array(reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok({
        let length = read_i64(reader)?;

        if length == -1 {
            RespData::NilArray
        } else {
            let length = length.try_into()?;
            let mut results = Vec::with_capacity(length);

            while results.len() < length {
                if let Some(item) = parse_resp(reader)? {
                    results.push(item);
                } else {
                    break;
                }
            }

            RespData::Array(results)
        }
    })
}

fn parse_resp_bulk_string(reader: &mut BufReader<&TcpStream>) -> Result<RespData, RespError> {
    Ok({
        let length = read_i64(reader)?;

        if length == -1 {
            RespData::NilBulkString
        } else {
            let length = length.try_into()?;
            let mut buf = vec![0; length];

            reader.read_exact(&mut buf)?;

            read_crlf(reader)?;

            RespData::BulkString(String::from_utf8(buf)?)
        }
    })
}

fn parse_resp_inline_command(
    reader: &mut BufReader<&TcpStream>,
    first: u8,
) -> Result<RespData, RespError> {
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

fn read_string(reader: &mut BufReader<&TcpStream>) -> Result<String, RespError> {
    let mut string = String::new();
    let n = reader.read_line(&mut string)?;

    if n > 0 {
        Ok(string)
    } else {
        Err(RespError::MissingCRLF) //Unexpected EOF
    }
}

fn read_i64(reader: &mut BufReader<&TcpStream>) -> Result<i64, RespError> {
    Ok(read_string(reader)?.parse::<i64>()?)
}

fn read_crlf(reader: &mut BufReader<&TcpStream>) -> Result<(), RespError> {
    let mut buf = [0; 2];

    reader.read_exact(&mut buf)?;

    if buf[0] == b'\r' && buf[1] == b'\n' {
        Ok(())
    } else {
        Err(RespError::MissingCRLF)
    }
}
