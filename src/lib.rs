use regex_automata::{Regex, RegexBuilder, DFA, DenseDFA};
use std::collections::VecDeque;

// state to keep for each Regex
struct Search<D: DFA> {
    regex: Regex<D>,
    state_id: D::ID,
    was_match: bool,
    is_match: bool,
}

#[derive(Debug)]
pub enum Error {
    InvalidRegex
}

/// A Ringbuffer backed steam searcher
/// 
/// Usage:
///  1. create a searcher using `new`.
/// 
///  2. Add all regexes to search for with `add_regex` or `add_regex_str`.
/// 
///  3. For every input byte:
///     Call `push` with the input, then call `matches` to obtain matches or `matches_string` to obtain match strings.
/// 
///  4. To get the input data for a match: call `match_data` or `match_string`.
///     This should happen before the next call to `push` to avoid overwriting the data for this match.
/// 
pub struct RingSearcher<D: DFA> {
    buffer: VecDeque<u8>,
    position: usize,
    searches: Vec<Search<D>>,
    buffer_size: usize,
}

impl<D: DFA> RingSearcher<D> {
    /// Create a ringbuffer backed regex stream searcher with the given ringbuffer size.
    /// The size should exeed the longest expected match.
    pub fn new(buffer_size: usize) -> Self {
        RingSearcher {
            searches: vec![],
            buffer: VecDeque::with_capacity(buffer_size),
            position: 0,
            buffer_size,
        }
    }

    /// add a Regex to search for
    pub fn add_regex(&mut self, regex: Regex<D>) {
        let state_id = regex.forward().start_state();
        self.searches.push(Search {
            state_id,
            is_match: regex.forward().is_match_state(state_id),
            regex,
            was_match: false,
        });
    }

    /// feed one stream byte to the searcher
    /// `matches` or `matches_string` must be called to obtain the matches ending at the *previous* input byte.
    pub fn push(&mut self, input: u8) {
        if self.buffer.len() + 1 > self.buffer_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(input);
        self.position += 1;

        for search in &mut self.searches {
            let dfa = search.regex.forward();
            let mut state_id = dfa.next_state(search.state_id, input);
            let is_match = dfa.is_match_state(state_id);

            if dfa.is_dead_state(state_id) {
                state_id = dfa.start_state();
            }

            // update state
            search.was_match = search.is_match;
            search.is_match = is_match;
            search.state_id = state_id;
        }
    }

    pub fn finish(&mut self) {

    }

    /// obtain the matches ending at the previous input byte
    pub fn matches(&self) -> impl Iterator<Item=(usize, Match)> + '_ {
        let position = self.position;
        self.searches.iter().enumerate().filter_map(move |(i, search)| {
            match (search.was_match, search.is_match) {
                (true, false) => {
                    search.regex.reverse().rfind_iter(self.buffer.iter().rev().cloned().skip(1)).map(move |len| {
                        let start = if len == self.buffer.len() {
                            None
                        } else {
                            Some(position - len - 1)
                        };

                        (i, Match {
                            start,
                            end: position - 1,
                        })
                    })
                }
                _ => None
            }
        })
    }

    /// Same as `matches` but returns the match string
    /// 
    /// Warning: This function allocates for every match.
    /// Use for debugging only.
    pub fn matches_string(&self) -> impl Iterator<Item=(usize, String)> + '_ {
        self.matches().map(move |(i, m)| (i, self.match_string(&m)))
    }

    /// Obtain the data for a specific match, as far as it is still in the buffer.
    /// Data is obtained as a pair of slices to avoid copying.
    pub fn match_data(&self, match_: &Match) -> (&[u8], &[u8]) {
        let (head, tail) = self.buffer.as_slices();

        // first data byte in self.buffer is at this stream position
        let offset = self.position - self.buffer.len();

        // position of match start in the buffer
        let start = match_.start.unwrap_or(offset) - offset;

        // position of match end in the buffer
        let end = match_.end - offset;
        
        (slice_window(head, start, end), slice_window(tail, start.saturating_sub(head.len()), end.saturating_sub(head.len())))
    }

    /// Obtain the String for a specific match, as far as it is still in the buffer.
    /// Warning: Allocates.
    pub fn match_string(&self, match_: &Match) -> String {
        let (head, tail) = self.match_data(match_);
        format!("{}{}", String::from_utf8_lossy(head), String::from_utf8_lossy(tail))
    }
}

impl RingSearcher<DenseDFA<Vec<usize>, usize>> {
    /// convinience function to add Regex from a `str`.
    pub fn add_regex_str(&mut self, regex_str: &str) -> Result<(), Error> {
        let regex = RegexBuilder::new().build(regex_str).map_err(|e| Error::InvalidRegex)?;
        self.add_regex(regex);
        Ok(())
    }
}

fn slice_window(slice: &[u8], start: usize, end: usize) -> &[u8] {
    &slice[start.min(slice.len()) .. end.min(slice.len())]
}

/// Match object.
/// 
/// Contains the stream positions of the match.
/// If the start of a match could not be found, `start` will be `None`.
#[derive(Copy, Clone, Debug)]
pub struct Match {
    pub start: Option<usize>,
    pub end: usize,
}
