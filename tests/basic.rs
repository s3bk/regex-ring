use regex_ring::RingSearcher;

#[test]
fn basic() {
    let input = "The lazy dog jumps over the brown fence.";
    
    let mut searcher = RingSearcher::new(1024);
    searcher.add_regex_str(r"d[a-z]+g").expect("failed to compile regex");
    searcher.add_regex_str(r"The").expect("failed to compile regex");
    searcher.add_regex_str(r"\.").expect("failed to compile regex");
    
    let mut expected = [
        // search id, start position, match string
        (1, 0, "The"),
        (0, 9, "dog"),
        (2, 39, ".")
    ].iter().cloned();
    
    searcher.input_matches(input.as_bytes(), |search_id, match_, data| {
        let (expected_id, expected_pos, expected_match_str) = expected.next().expect("too many matches");
        assert_eq!(expected_id, search_id);
        assert_eq!(expected_pos, match_.start.expect("should have a start"));
        assert_eq!(data, *expected_match_str.as_bytes());
        assert_eq!(expected_pos + expected_match_str.len(), match_.end);
    });

    assert!(expected.next().is_none());
}