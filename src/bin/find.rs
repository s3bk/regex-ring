use std::{env, io, str};
use std::io::Read;

use regex_automata::RegexBuilder;
use regex_ring::{RingSearcher, Match};

fn main() {
    let mut args = env::args();
    let _ = args.next().expect("no program name");
    let regex_s = args.next().expect("no regex");

    let regex = RegexBuilder::new().build(&regex_s).expect("invalid regex");
    let mut searcher = RingSearcher::new(regex, 1024 * 1024);

    for b in io::stdin().lock().bytes() {
        if let Some(m) = searcher.push(b.unwrap()) {
            println!("{:?}", m);
            let (head, tail) = searcher.match_data(&m);
            if let (Ok(a), Ok(b)) = (str::from_utf8(head), str::from_utf8(tail)) {
                println!("> {}{}", a, b);
            }
        }
    }
}