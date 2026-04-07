/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use log::{info, trace, warn};
use nix::libc;
use std::fs::File;
use std::fs::ReadDir;
use std::io::Error;
mod commands;
mod io;
mod map;
mod msgs;

pub struct Rpmsgfs {
    rpmsgfs_io: io::Io,
    files: map::Map<File>,
    directories: map::Map<ReadDir>,
}

impl Rpmsgfs {
    pub fn new(device_filename: String) -> Rpmsgfs {
        Rpmsgfs {
            rpmsgfs_io: io::Io::new(device_filename),
            files: map::Map::new(),
            directories: map::Map::new(),
        }
    }

    pub fn process_command(&mut self) {
        let (buf, size) = self.rpmsgfs_io.read_packet();
        info!("Recv msg: {:?}", &buf[..size]);

        let header: msgs::Header = bincode::deserialize(&buf).unwrap();
        trace!("cmd:{:} cookie:0x{:x}", header.command, header.cookie);
        let data_offset = std::mem::size_of::<msgs::Header>();
        let data = &buf[data_offset..];
        let result: Result<(i32, Vec<u8>), _> = match header.command {
            msgs::CMD_OPEN => commands::open(&mut self.files, &data),
            msgs::CMD_CLOSE => commands::close(&mut self.files, &data),
            msgs::CMD_READ => commands::read(&mut self.files, &mut self.rpmsgfs_io, &header, &data),
            msgs::CMD_WRITE => commands::write(&mut self.files, &header, &data),
            msgs::CMD_SEEK => commands::seek(&mut self.files, &data),
            //msgs::CMD_IOCTL
            msgs::CMD_SYNC => commands::sync(&mut self.files, &data),
            //msgs::CMD_DUP
            msgs::CMD_FSTAT => commands::fstat(&mut self.files, &data),
            msgs::CMD_FTRUNCATE => commands::ftruncate(&mut self.files, &data),
            msgs::CMD_OPENDIR => commands::opendir(&mut self.directories, &data),
            msgs::CMD_READDIR => commands::readdir(&mut self.directories, &data),
            msgs::CMD_REWINDDIR => commands::rewinddir(&mut self.directories, &data),
            msgs::CMD_CLOSEDIR => commands::closedir(&mut self.directories, &data),
            msgs::CMD_STATFS => commands::statfs(&data),
            msgs::CMD_UNLINK => commands::unlink(&data),
            msgs::CMD_MKDIR => commands::mkdir(&data),
            msgs::CMD_RMDIR => commands::rmdir(&data),
            msgs::CMD_RENAME => commands::rename(&data),
            msgs::CMD_STAT => commands::stat(&data),
            msgs::CMD_FCHSTAT => commands::fchstat(&mut self.files, &data),
            msgs::CMD_CHSTAT => commands::chstat(&data),
            _ => Err(Error::from_raw_os_error(-libc::ENOTSUP)),
        };
        match result {
            Ok((result, response_data)) => {
                if result != commands::RESULT_DO_NOT_SEND_RESPONSE {
                    self.rpmsgfs_io
                        .send_response(&header, result, response_data)
                        .expect("Cannot send response to rpmsg characted device");
                } else {
                }
            }

            Err(e) => {
                let os_error = match e.raw_os_error() {
                    Some(i) => i,
                    None => libc::EIO,
                };
                warn!("Respond error: os_error={}, details={}", os_error, e);
                self.rpmsgfs_io
                    .send_response(&header, -os_error, vec![])
                    .expect("Cannot send response to rpmsg characted device");
            }
        };
    }
}
