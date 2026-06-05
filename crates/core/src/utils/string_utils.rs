use rand::distributions::{Alphanumeric, DistString};

pub fn quote_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn random_str(lenght: Option<usize>) -> String {
    let len = lenght.unwrap_or(32);

    Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}
