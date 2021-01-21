use std::{env, io, str};
use std::io::Read;

use regex_ring::{RingSearcher};

fn main() {
    let mut args = env::args();
    let _ = args.next().expect("no program name");

    let mut searcher = RingSearcher::new(1024);

    for regex_str in args {
        searcher.add_regex_str(&regex_str).expect("invalid regex");
    }

    searcher.input_matches(io::stdin().lock().bytes().flat_map(Result::ok), |search_id, match_, match_data| {
        println!("#{} {:?}", search_id, match_);
        println!("> {}", match_data.to_string());
    });
}