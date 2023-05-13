use crate::{resp, store::Store};

pub fn get(store: &dyn Store, ctx: &Vec<resp::Data>) -> Vec<u8> {
    if let Some(resp::Data::String(key)) = ctx.get(1) {
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

pub fn set(store: &mut dyn Store, ctx: &Vec<resp::Data>) -> Vec<u8> {
    if let Some(resp::Data::String(key)) = ctx.get(1) {
        if let Some(resp::Data::String(value)) = ctx.get(2) {
            println!("cmd: SET, key: {}, value: {}", key, value);

            store.set(key, value.to_string());

            return resp::ser_string("OK");
        }

        println!("cmd: SET, key: {}, No value provided", key);
        return resp::ser_error("No value provided");
    }

    println!("cmd: SET, No key");
    resp::ser_error("No key provided")
}

pub fn del(store: &mut dyn Store, ctx: &Vec<resp::Data>) -> Vec<u8> {
    let keys = ctx[1..].iter().fold(Vec::new(), |mut acc, curr| {
        if let resp::Data::String(curr) = curr {
            acc.push(curr);
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
