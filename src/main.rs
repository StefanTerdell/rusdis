mod commands;
mod resp;
mod store;

use std::{io::Write, net::TcpListener};

fn main() {
    let mut store = store::HashMapStore::new();
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut allow_pipeline = true;

        println!("stream open, parsing buffer");

        loop {
            let message = resp::parse(&mut stream, allow_pipeline);
            allow_pipeline = false;

            if let Ok(Some(message)) = message {
                if let resp::Data::Array(array) = message {
                    if let Some(resp::Data::String(cmd)) = array.get(0) {
                        let res = match cmd.as_str() {
                            "PING" => commands::ping(),
                            "SET" => commands::set(&mut store, &array),
                            "GET" => commands::get(&store, &array),
                            "DEL" => commands::del(&mut store, &array),
                            _ => resp::ser_error("Unknown command"),
                        };

                        stream.write_all(&res).unwrap();
                        stream.flush().unwrap();

                        println!(
                            "Sent '{}'",
                            String::from_utf8(res[..res.len() - 2].to_vec())
                                .unwrap()
                                .replace("\r\n", "\\r\\n")
                        );

                        continue;
                    }
                }
            }

            break;
        }

        println!("stream closing");
    }
}
