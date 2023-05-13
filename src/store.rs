pub trait Store {
    fn get(&self, key: &str) -> Option<&String>;
    fn set(&mut self, key: &str, value: String);
    fn del(&mut self, keys: &[&String]) -> i64;
}

pub struct HashMapStore {
    data: std::collections::HashMap<String, String>,
}

impl HashMapStore {
    pub fn new() -> HashMapStore {
        HashMapStore {
            data: std::collections::HashMap::new(),
        }
    }
}

impl Store for HashMapStore {
    fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    fn set(&mut self, key: &str, value: String) {
        self.data.insert(key.to_owned(), value);
    }

    fn del(&mut self, keys: &[&String]) -> i64 {
        keys.iter()
            .map(|key| {
                if self.data.contains_key(*key) {
                    self.data.remove(*key);
                    1
                } else {
                    0
                }
            })
            .sum()
    }
}
