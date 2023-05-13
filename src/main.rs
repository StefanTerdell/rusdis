mod commands;
mod resp;
mod store;

use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

fn main() {
    let mut store = store::HashMapStore::new();
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        println!("stream open, parsing buffer");

        let message = resp::parse(&mut stream, true);

        if let Ok(Some(message)) = message {
            if let resp::Data::Array(arr) = message {
                handle_array(arr, &mut store, &mut stream)
            }
        }

        println!("stream closing");
    }
}

fn handle_array(arr: Vec<resp::Data>, store: &mut dyn store::Store, stream: &mut TcpStream) {
    if let Some(cmd) = commands::get_arg(&arr, 0) {
        let res = match cmd.as_str() {
            "PING" => commands::ping(),
            "SET" => commands::set(store, &arr),
            "GET" => commands::get(store, &arr),
            "DEL" => commands::del(store, &arr),
            _ => resp::ser_error("Unknown command"),
        };

        stream.write_all(&res).unwrap();
        stream.flush().unwrap();

        println!(
            "Sent '{}'",
            String::from_utf8(res).unwrap().replace("\r\n", "\\r\\n")
        );
    } else {
        for item in arr {
            if let resp::Data::Array(inner) = item {
                handle_array(inner, store, stream);
            }
        }
    }
}
