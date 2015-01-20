// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name="reprint"]
#![feature(slicing_syntax)]
#![allow(unstable)]

use std::io::{File, FileMode, FileAccess};
use std::path::GenericPath;
use std::io::fs::{self, PathExtensions};


pub struct Change {
    start_byte: u32,
    end_byte: u32,
    text: String
}

pub type ChangeSet = Vec<Change>;

pub fn reprint(file: &Path, mut changes: ChangeSet) {
    changes.sort();

    if let Err(msg) = verify(&changes) {
        println!("Verification error: {}",  msg);
        return;
    }

    let input = match read_file(file) {
        Ok(i) => i,
        Err(msg) => {
            println!("Error reading file: {}",  msg);
            return;
        }
    };

    let changes_size = changes.iter().fold(0i64, |a, c| a + c.delta());
    let mut buf: Vec<u8> = Vec::with_capacity((input.as_bytes().len() as i64 +
                                               changes_size) as usize);
    match process(input, changes, &mut buf) {
        Ok(()) => {
            if let Err(msg) = write_file(file, buf) {
                println!("Error writing file: {}",  msg);
                return;
            }
        }
        Err(msg) => {
            println!("Error processing changes: {}",  msg);
            return;
        }
    }

    // Success!
}

// Assumes changes is sorted.
fn verify(changes: &ChangeSet) -> Result<(), String> {
    let mut prev_start = 0;
    let mut prev_end = 0;
    for ch in changes.iter() {
        if ch.end_byte < ch.start_byte {
            return Err(format!("Bad change at {}", ch.start_byte));
        }
        if ch.start_byte < prev_end {
            return Err(format!("Overlapping changes: {}--{} overlaps {}--{} ",
                               prev_start,
                               prev_end,
                               ch.start_byte,
                               ch.end_byte));            
        }
        prev_start = ch.start_byte;
        prev_end = ch.end_byte;
    }

    Ok(())
}

fn read_file(file: &Path) -> Result<String, String> {
    let file = File::open(file);
    let mut file = match file {
        Ok(f) => f,
        Err(e) => return Err(e.desc.to_string())
    };

    match file.read_to_string() {
        Ok(contents) => Ok(contents),
        Err(e) => Err(e.desc.to_string())
    }
}

// precondition: changes == changes.sort() && verify(changes)
fn process(input: String,
           changes: ChangeSet,
           buf: &mut Vec<u8>)
-> Result<(), String> {
    let input = input.as_bytes();
    // Current position in the input.
    let mut in_pos = 0us;
    for ch in changes.iter() {
        if in_pos >= input.len() {
            return Err(format!("Input out of range. {} >= {}", in_pos, input.len()));
        }
        if ch.start_byte as usize >= input.len() {
            return Err(format!("Change out of range for input. {} >= {}",
                               ch.start_byte,
                               input.len()));
        }
        buf.push_all(&input[in_pos..ch.start_byte as usize]);

        let text = ch.text.as_bytes();
        buf.push_all(text);
        in_pos = ch.end_byte as usize;
    }

    // Push the rest of the input onto the output.
    buf.push_all(&input[in_pos..]);
    Ok(())
}

fn write_file(input_path: &Path, buf: Vec<u8>) -> Result<(), String> {
    // Prepare file names.
    let input_name = match input_path.as_str() {
        Some(n) => n.to_string(),
        None => return Err(format!("Couldn't turn path '{:?}' into a string", input_path))
    };

    let tmp_path = Path::new(input_name.clone() + ".tmp");
    let bk_path = Path::new(input_name.clone() + ".bk");
    if tmp_path.exists() {
        return Err(format!("File '{:?}' already exists", tmp_path))
    }
    if bk_path.exists() {
        return Err(format!("File '{:?}' already exists", bk_path))
    }

    // Write to temporary file.
    let mut tmp_file = match File::open_mode(&tmp_path,
                                             FileMode::Open,
                                             FileAccess::Write) {
        Ok(f) => f,
        Err(e) => return Err(format!("Couldn't open '{:?}': {}", tmp_path, e.desc))
    };
    match tmp_file.write(&buf[]) {
        Ok(()) => {}
        Err(e) => return Err(format!("Couldn't write to '{:?}': {}", tmp_path, e.desc))
    }

    // Rename input file to backup.
    match fs::rename(input_path, &bk_path) {
        Ok(()) => {},
        Err(e) => return Err(format!("Couldn't rename '{:?}' to '{:?}': {}",
                                     input_path,
                                     bk_path,
                                     e.desc))
    }

    // Rename temp file to input file.
    match fs::rename(&tmp_path, input_path) {
        Ok(()) => {},
        Err(e) => return Err(format!("Couldn't rename '{:?}' to '{:?}': {}",
                                     tmp_path,
                                     input_path,
                                     e.desc))
    }

    // Success!
    Ok(())
}

impl PartialEq for Change {
    fn eq(&self, other: &Change) -> bool {
        self.start_byte == other.start_byte
    }
}

impl Eq for Change {}

impl Ord for Change {
    fn cmp(&self, other: &Change) -> std::cmp::Ordering {
        self.start_byte.cmp(&other.start_byte)
    }
}

impl PartialOrd for Change {
    fn partial_cmp(&self, other: &Change) -> Option<std::cmp::Ordering> {
        self.start_byte.partial_cmp(&other.start_byte)
    }
}

impl Change {
    pub fn new(start_byte: u32, end_byte: u32, text: String) -> Change {
        Change {
            start_byte: start_byte,
            end_byte: end_byte,
            text: text
        }
    }

    fn delta(&self) -> i64 {
        self.text.as_bytes().len() as i64 -
            (self.end_byte as i64 - self.start_byte as i64)
    }
}

fn main() {
    let path = Path::new("/home/ncameron/reprint/data/hello.rs");
    let change = Change::new(3, 8, "Goodbye cruel".to_string());
    reprint(&path, vec![change]);
}
