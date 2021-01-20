use regex_automata::{Regex, DFA};
use std::collections::VecDeque;

pub struct RingSearcher<D: DFA> {
    regex: Regex<D>,
    buffer: VecDeque<u8>,
    state_id: D::ID,
    was_match: bool,
    position: usize,
}

impl<D: DFA> RingSearcher<D> {
    pub fn new(regex: Regex<D>, buffer_size: usize) -> Self {
        RingSearcher {
            state_id: regex.forward().start_state(),
            regex,
            buffer: VecDeque::with_capacity(buffer_size),
            was_match: false,
            position: 0,
        }
    }

    pub fn push(&mut self, input: u8) -> Option<Match> {
        let capacity = self.buffer.capacity();
        self.buffer.truncate(capacity - 1);
        self.buffer.push_back(input);

        let dfa = self.regex.forward();
        let mut state_id = dfa.next_state(self.state_id, input);
        let is_match = dfa.is_match_state(state_id);
        let was_match = self.was_match;
        let position = self.position;

        if dfa.is_dead_state(state_id) {
            state_id = dfa.start_state();
        }

        // update state
        self.was_match = is_match;
        self.state_id = state_id;
        self.position += 1;

        match (was_match, is_match) {
            (true, false) => {
                self.regex.reverse().rfind_iter(self.buffer.iter().rev().cloned().skip(1)).map(move |len| {
                    let start = if len == self.buffer.len() {
                        None
                    } else {
                        Some(position - len)
                    };

                    Match {
                        start,
                        end: position,
                    }
                })
            }
            _ => None
        }
    }

    pub fn match_data(&self, match_: &Match) -> (&[u8], &[u8]) {
        let (head, tail) = self.buffer.as_slices();

        // first data byte in self.buffer is at this stream position
        let offset = self.position - self.buffer.len();

        // position of match start in the buffer
        let start = match_.start.unwrap_or(offset) - offset;

        // position of match end in the buffer
        let end = match_.end - offset;
        
        (slice_window(head, start, end), slice_window(tail, start, end))
    }
}

fn slice_window(slice: &[u8], start: usize, end: usize) -> &[u8] {
    &slice[start.min(slice.len()) .. end.min(slice.len())]
}

#[derive(Copy, Clone, Debug)]
pub struct Match {
    pub start: Option<usize>,
    pub end: usize,
}
