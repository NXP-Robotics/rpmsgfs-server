/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use log::info;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use crate::rpmsgfs::msgs;

pub struct Io {
    rpmsg_character_device: File,
}

impl Io {
    pub fn new(device_filename: &Path) -> Io {
        info!("open {:?}", device_filename);
        let mut f = OpenOptions::new()
            .append(true)
            .read(true)
            .open(device_filename)
            .expect("rpmsg device not found");
        f.write_all(b"Hello").expect("Cannot write");
        Io {
            rpmsg_character_device: f,
        }
    }

    pub fn read_packet(&mut self) -> ([u8; 2000], usize) {
        let mut buf: [u8; 2000] = [0; 2000];
        let n = self
            .rpmsg_character_device
            .read(&mut buf)
            .expect("Cannot read from rpmsg device");
        (buf, n)
    }

    pub fn send_response(
        &mut self,
        header: &msgs::Header,
        result: i32,
        data: Vec<u8>,
    ) -> Result<usize, std::io::Error> {
        let header = bincode::serialize(&msgs::Header {
            command: header.command,
            result: result,
            cookie: header.cookie,
        })
        .expect("Cannot serialize header");

        let binding = [header, data].concat();
        let combined = binding.as_slice();
        info!("Send {:?}", combined);
        self.rpmsg_character_device.write(combined)
    }
}
