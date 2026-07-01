#[cfg(test)]
mod tests {
    #[test]
    fn debug_token_counts() {
        let enc = tiktoken_rs::cl100k_base().unwrap();
        let hello = enc.encode("hello").unwrap().len();
        let helloworld = enc.encode("hello world").unwrap().len();
        let test = enc.encode("test").unwrap().len();
        let empty = enc.encode("").unwrap().len();
        println!("hello: {}", hello);
        println!("hello world: {}", helloworld);
        println!("test: {}", test);
        println!("empty: {}", empty);
        assert!(false, "check output");
    }
}
