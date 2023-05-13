mod commands;
mod resp;
mod store;

use async_recursion::async_recursion;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let mut store = store::HashMapStore::new();
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (mut stream, _) = listener.accept().await.unwrap();

        println!("stream open, parsing buffer");

        let message = resp::parse(&mut stream, true);

        if let Ok(Some(resp::Data::Array(arr))) = message.await {
            handle_array(arr, &mut store, &mut stream).await;
        }

        println!("stream closing");
    }
}

#[async_recursion(?Send)]
async fn handle_array(arr: Vec<resp::Data>, store: &mut dyn store::Store, stream: &mut TcpStream) {
    if let Some(cmd) = commands::get_arg(&arr, 0) {
        let res = match cmd.as_str() {
            "PING" => commands::ping(),
            "SET" => commands::set(store, &arr),
            "GET" => commands::get(store, &arr),
            "DEL" => commands::del(store, &arr),
            _ => resp::ser_error("Unknown command"),
        };

        stream.write_all(&res).await.unwrap();
        stream.flush().await.unwrap();

        println!(
            "Sent '{}'",
            String::from_utf8(res).unwrap().replace("\r\n", "\\r\\n")
        );
    } else {
        for item in arr {
            if let resp::Data::Array(inner) = item {
                handle_array(inner, store, stream).await;
            }
        }
    }
}
