use regex_automata::{Regex, RegexBuilder, DFA, DenseDFA};
use std::collections::VecDeque;
use std::borrow::Borrow;

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
///     Call `push` with the input, then call `matches` to obtain matches.
/// 
///  4. To get the input data for a match: call `match_data`.
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
    /// 
    /// Returns the identifier for this search.
    /// The identifiers will be 0, 1, ...
    pub fn add_regex(&mut self, regex: Regex<D>) -> usize {
        let state_id = regex.forward().start_state();
        let search_nr = self.searches.len();
        self.searches.push(Search {
            state_id,
            regex,
            is_match: false, // first input byte requires this to work.
            was_match: false,
        });
        search_nr
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

    /// Obtain the matches ending at the previous input byte.
    /// 
    /// The iterator yields (search identifier, match).
    pub fn matches(&self) -> impl Iterator<Item=(usize, Match)> + '_ {
        let position = self.position;
        self.searches.iter().enumerate().filter_map(move |(i, search)| {
            if (search.was_match, search.is_match) == (true, false) {
                rfind_iter(search.regex.reverse(), self.buffer.iter().rev().cloned().skip(1)).map(move |len| {
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
            } else {
                None
            }
        })
    }

    /// Obtain the final matches.
    /// 
    /// This will return the matches ending at the last input byte and should only be called when no more input follows.
    /// The iterator yields (search identifier, match).
    pub fn final_matches(&self) -> impl Iterator<Item=(usize, Match)> + '_ {
        let position = self.position;
        self.searches.iter().enumerate().filter_map(move |(i, search)| {
            if search.is_match {
                rfind_iter(search.regex.reverse(), self.buffer.iter().rev().cloned()).map(move |len| {
                    let start = if len == self.buffer.len() {
                        None
                    } else {
                        Some(position - len)
                    };

                    (i, Match {
                        start,
                        end: position,
                    })
                })
            } else {
                None
            }
        })
    }


    /// Obtain the data for a specific match, as far as it is still in the buffer.
    /// Data is obtained as a pair of slices to avoid copying.
    pub fn match_data(&self, match_: &Match) -> MatchData {
        let (head, tail) = self.buffer.as_slices();

        // first data byte in self.buffer is at this stream position
        let offset = self.position - self.buffer.len();

        // position of match start in the buffer
        let start = match_.start.unwrap_or(offset) - offset;

        // position of match end in the buffer
        let end = match_.end - offset;
        
        MatchData {
            head: slice_window(head, start, end),
            tail: slice_window(tail, start.saturating_sub(head.len()), end.saturating_sub(head.len()))
        }
    }

    /// Perform matching on the entire input iterator and call `callback` for every match.
    /// 
    /// The callback recieves:
    ///  - search id
    ///  - the match
    ///  - the match data
    pub fn input_matches<I, V, F>(&mut self, input: I, mut callback: F)
        where I: IntoIterator<Item=V>, V: Borrow<u8>, F: FnMut(usize, &Match, MatchData)
    {
        for b in input.into_iter() {
            self.push(*b.borrow());
            for (re_nr, match_) in self.matches() {
                let data = self.match_data(&match_);
                callback(re_nr, &match_, data);
            }
        }

        for (re_nr, match_) in self.final_matches() {
            let data = self.match_data(&match_);
            callback(re_nr, &match_, data);
        }
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


/// Works like rfind, but returns the number of bytes in the reverse direction and takes an iterator input.
fn rfind_iter<D: DFA>(dfa: &D, bytes: impl Iterator<Item=u8>) -> Option<usize> {
    let mut state = dfa.start_state();
    let mut last_match = if dfa.is_dead_state(state) {
        return None;
    } else if dfa.is_match_state(state) {
        Some(0)
    } else {
        None
    };
    for (i, b) in bytes.enumerate() {
        state = unsafe { dfa.next_state_unchecked(state, b) };
        if dfa.is_match_or_dead_state(state) {
            if dfa.is_dead_state(state) {
                return last_match;
            }
            last_match = Some(i + 1);
        }
    }
    last_match
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

/// Input data for a Match.
/// 
/// Internally composed of two slices into the ringbuffer.
#[derive(Copy, Clone, Debug)]
pub struct MatchData<'a> {
    pub head: &'a [u8],
    pub tail: &'a [u8],
}
impl<'a> MatchData<'a> {
    /// Obtain the String for this match data.
    /// 
    /// Warning: Allocates.
    pub fn to_string(&self) -> String {
        format!("{}{}", String::from_utf8_lossy(self.head), String::from_utf8_lossy(self.tail))
    }

    /// Obtain the data of this match as a `Vec<u8>`
    pub fn to_vec(&self) -> Vec<u8> {
        [self.head, self.tail].concat()
    }

    /// Length of match data
    pub fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }
}

impl<'a> PartialEq<[u8]> for MatchData<'a> {
    fn eq(&self, rhs: &[u8]) -> bool {
        let MatchData { head, tail } = *self;
        if self.len() != rhs.len() {
            return false;
        }
        let (rhs_head, rhs_tail) = rhs.split_at(head.len());
        head == rhs_head && tail == rhs_tail
    }
}