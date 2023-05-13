use crate::{resp, store::Store};

pub fn get_arg(args: &Vec<resp::Data>, index: usize) -> Option<String> {
    match args.get(index) {
        Some(resp::Data::String(str) | resp::Data::BulkString(str)) => Some(str.to_string()),
        _ => None,
    }
}

pub fn get(store: &dyn Store, args: &Vec<resp::Data>) -> Vec<u8> {
    if let Some(key) = get_arg(args, 1) {
        let data = store.get(&key);

        if let Some(data) = data {
            println!("cmd: GET, key: {}, value: {}", key, data);
            return resp::ser_bulk_string(data);
        };

        println!("cmd: GET, key: {}, value null", key);
        return resp::ser_null_bulk_string();
    }

    println!("cmd: GET, no key");
    resp::ser(resp::Data::Error(String::from("No key provided")))
}

pub fn set(store: &mut dyn Store, args: &Vec<resp::Data>) -> Vec<u8> {
    if let Some(key) = get_arg(args, 1) {
        if let Some(value) = get_arg(args, 2) {
            println!("cmd: SET, key: {}, value: {}", key, value);

            store.set(&key, value.to_string());

            return resp::ser_string("OK");
        }

        println!("cmd: SET, key: {}, No value provided", key);
        return resp::ser_error("No value provided");
    }

    println!("cmd: SET, No key");
    resp::ser_error("No key provided")
}

pub fn del(store: &mut dyn Store, args: &Vec<resp::Data>) -> Vec<u8> {
    let keys = args[1..].iter().fold(Vec::new(), |mut acc, curr| {
        if let resp::Data::String(str) | resp::Data::BulkString(str) = curr {
            acc.push(str)
        }

        acc
    });

    let deleted_lines = store.del(&keys);

    println!("cmd: DEL, keys: {:?}, deleted: {}", keys, deleted_lines);
    resp::ser_int(deleted_lines)
}

pub fn ping() -> Vec<u8> {
    println!("cmd: PING,");
    resp::ser_string("PONG")
}
