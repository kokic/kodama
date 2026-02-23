// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct Counter {
    pub numbers: Vec<u8>,
}

impl Counter {
    pub fn init() -> Self {
        Counter { numbers: vec![0] }
    }

    pub fn display(&self) -> String {
        let mut out = String::new();
        for number in &self.numbers {
            let _ = write!(out, "{}.", number);
        }
        out
    }

    pub fn step_at_mut(&mut self, level: usize) {
        let len = self.numbers.len();
        let index = len - level;
        if index < len {
            self.numbers[index] += 1;
        }
    }

    pub fn step_mut(&mut self) {
        self.step_at_mut(1)
    }

    pub fn left_shift_by(&self, n: u8) -> Counter {
        let mut counter = self.clone();
        counter.numbers.push(n);
        counter
    }

    pub fn left_shift(&self) -> Counter {
        self.left_shift_by(0)
    }
}
