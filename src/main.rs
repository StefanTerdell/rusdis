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
        let mut allow_pipeline = true;

        println!("stream open, parsing buffer");

        loop {
            let result = parse_resp(&mut reader, allow_pipeline);

            let (done, message) = match result {
                Ok(response) => match response {
                    Some(value) => (false, format!("value: {:?}", value)),
                    None => (true, String::from("buffer empty")),
                },
                Err(error) => (true, format!("error: {:?}", error)),
            };

            println!("{message}");
            
            writer.write(message.as_bytes()).unwrap();

            allow_pipeline = false;

            if done {
                break;
            }
        }

        println!("stream closing");
    }
}
