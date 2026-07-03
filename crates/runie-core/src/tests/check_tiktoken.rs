#[cfg(test)]
mod tests {
    use crate::tokens::tiktoken_count;
    
    #[test]
    fn check_counts() {
        println!("empty: {:?}", tiktoken_count(""));
        println!("abcd: {:?}", tiktoken_count("abcd"));
        println!("abcdefgh: {:?}", tiktoken_count("abcdefgh"));
        println!("hello: {:?}", tiktoken_count("hello"));
        println!("goodbye: {:?}", tiktoken_count("goodbye"));
    }
}
