use std::{env, io, str};
use std::io::Read;

use regex_automata::RegexBuilder;
use regex_ring::{RingSearcher, Match};

fn main() {
    let mut args = env::args();
    let _ = args.next().expect("no program name");

    let mut searcher = RingSearcher::new(1024);

    for regex_str in args {
        searcher.add_regex_str(&regex_str).expect("invalid regex");
    }

    for b in io::stdin().lock().bytes() {
        searcher.push(b.unwrap());
        for (i, m) in searcher.matches() {
            println!("#{} {:?}", i, m);
            let (head, tail) = searcher.match_data(&m);
            if let (Ok(a), Ok(b)) = (str::from_utf8(head), str::from_utf8(tail)) {
                println!("> {}{}", a, b);
            }
        }
    }
}