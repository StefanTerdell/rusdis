use rusdis::resp;

mod commands;
mod store;

use async_recursion::async_recursion;
use std::sync::Arc;
use store::HashMapStore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let store = Arc::new(RwLock::new(store::HashMapStore::new()));
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (mut stream, address) = listener.accept().await.unwrap();
        println!("New TCP connection to {}", address);
        let store = Arc::clone(&store);

        tokio::spawn(async move {
            let mut buffer = [0; 1024];

            loop {
                match stream.read(&mut buffer).await {
                    Ok(n) if n == 0 => {
                        // connection was closed
                        println!("Connection closed from {}", address);
                        break;
                    }
                    Ok(n) => {
                        let message = resp::parse(&mut buffer[..n].iter(), true);

                        let mut results = Vec::new();

                        if let Ok(Some(resp::Data::Array(arr))) = message {
                            execute_commands(arr, Arc::clone(&store), &mut results).await;

                            stream.write_all(&results).await.unwrap();
                            stream.flush().await.unwrap();

                            println!(
                                "Sent {} to {}",
                                String::from_utf8(results)
                                    .unwrap()
                                    .replace("\r\n", "\\r\\n"),
                                address
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        break;
                    }
                }
            }
        });
    }
}

#[async_recursion]
async fn execute_commands(
    arr: Vec<resp::Data>,
    store: Arc<RwLock<HashMapStore>>,
    acc: &mut Vec<u8>,
) {
    if let Some(cmd) = commands::get_arg(&arr, 0) {
        let res = match cmd.as_str() {
            "PING" => commands::ping(),
            "SET" => {
                let mut store_lock = store.write().await;
                commands::set(&mut *store_lock, &arr)
            }
            "GET" => {
                let store_lock = store.read().await;
                commands::get(&*store_lock, &arr)
            }
            "DEL" => {
                let mut store_lock = store.write().await;
                commands::del(&mut *store_lock, &arr)
            }
            _ => resp::ser_error("Unknown command"),
        };

        acc.extend(&res);
    } else {
        for item in arr {
            if let resp::Data::Array(inner) = item {
                execute_commands(inner, Arc::clone(&store), acc).await;
            }
        }
    }
}
