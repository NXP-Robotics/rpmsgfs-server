/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use std::collections::HashMap;

pub struct Map<T> {
    buf: HashMap<i32, (T, String)>,
}

impl<T> Map<T> {
    pub fn new() -> Map<T> {
        Map {
            buf: HashMap::new(),
        }
    }

    pub fn add(&mut self, dir: T, path: String) -> i32 {
        let id: i32 = match self.buf.keys().max() {
            Some(higest_id) => higest_id + 1,
            None => 1,
        };
        self.buf.insert(id, (dir, path.to_string()));
        id
    }

    pub fn get_mut(&mut self, id: i32) -> Result<&mut (T, String), std::io::Error> {
        match self.buf.get_mut(&id) {
            Some(item) => Ok(item),
            None => Err(std::io::Error::from_raw_os_error(nix::libc::EBADF)),
        }
    }

    pub fn remove(&mut self, id: i32) -> Result<(T, String), std::io::Error> {
        match self.buf.remove(&id) {
            Some(item) => Ok(item),
            None => Err(std::io::Error::from_raw_os_error(nix::libc::EBADF)),
        }
    }
}
