#[cfg(test)]
mod glob_test {
    #[test]
    fn test_patterns() {
        use glob::Pattern;
        let path = "/home/user/.kube/config";
        let p = Pattern::new("**/.kube/config").unwrap();
        assert!(p.matches(path), "Pattern **/.kube/config should match {}", path);
        
        let path2 = "/home/user/.docker/config.json";
        let p2 = Pattern::new("**/.docker/config.json").unwrap();
        assert!(p2.matches(path2), "Pattern **/.docker/config.json should match {}", path2);
    }
}
