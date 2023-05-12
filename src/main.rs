mod resp;

use resp::*;
use std::{
    io::{BufReader, BufWriter, Write},
    net::TcpListener,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        while let Ok(Some(response)) = parse_resp(&mut reader) {
            let debug = format!("response: {:?}", response);

            println!("{debug}");

            writer.write(debug.as_bytes()).unwrap();
        }

        writer.flush().unwrap();
    }
}
