/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use serde_derive::{Deserialize, Serialize};

// commands
pub const CMD_OPEN: u32 = 1;
pub const CMD_CLOSE: u32 = 2;
pub const CMD_READ: u32 = 3;
pub const CMD_WRITE: u32 = 4;
pub const CMD_SEEK: u32 = 5;
pub const _CMD_IOCTL: u32 = 6;
pub const CMD_SYNC: u32 = 7;
pub const _CMD_DUP: u32 = 8;
pub const CMD_FSTAT: u32 = 9;
pub const CMD_FTRUNCATE: u32 = 10;
pub const CMD_OPENDIR: u32 = 11;
pub const CMD_READDIR: u32 = 12;
pub const CMD_REWINDDIR: u32 = 13;
pub const CMD_CLOSEDIR: u32 = 14;
pub const CMD_STATFS: u32 = 15;
pub const CMD_UNLINK: u32 = 16;
pub const CMD_MKDIR: u32 = 17;
pub const CMD_RMDIR: u32 = 18;
pub const CMD_RENAME: u32 = 19;
pub const CMD_STAT: u32 = 20;
pub const CMD_FCHSTAT: u32 = 21;
pub const CMD_CHSTAT: u32 = 22;

// open file flags
pub const O_READ: i32 = 1 << 0;
pub const O_WRITE: i32 = 1 << 1;
pub const O_CREAT: i32 = 1 << 2;
pub const O_EXCL: i32 = 1 << 3;
pub const O_APPEND: i32 = 1 << 4;
pub const O_TRUNC: i32 = 1 << 5;
pub const O_NONBLOCK: i32 = 1 << 6;
pub const O_SYNC: i32 = 1 << 7;
pub const _O_BINARY: i32 = 1 << 8; // is always active in rust
pub const O_DIRECT: i32 = 1 << 9;
pub const O_DIRECTORY: i32 = 1 << 11;
pub const O_NOFOLLOW: i32 = 1 << 12;
pub const O_LARGEFILE: i32 = 1 << 13;
pub const O_NOATIME: i32 = 1 << 18;

// file types
pub const DT_UNKNOWN: u32 = 0;
pub const DT_FIFO: u32 = 1;
pub const DT_CHR: u32 = 2;
pub const _DT_SEM: u32 = 3;
pub const DT_DIR: u32 = 4;
pub const _DT_MQ: u32 = 5;
pub const DT_BLK: u32 = 6;
pub const _DT_SHM: u32 = 7;
pub const DT_REG: u32 = 8;
pub const _DT_MTD: u32 = 9;
pub const DT_LNK: u32 = 10;
pub const DT_SOCK: u32 = 12;

#[derive(Serialize, Deserialize)]
pub struct Header {
    pub command: u32,
    pub result: i32,
    pub cookie: u64,
}

#[derive(Deserialize)]
pub struct Open {
    pub flags: i32,
    pub mode: u32,
    //pathname: [u8],
}

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    // read and write
    pub fd: i32,
    pub content_size: u32,
    //pathname: [u8],
}

#[derive(Deserialize)]
pub struct Seek {
    pub fd: i32,
    pub whence: i32,
    pub offset: i32,
}

#[derive(Deserialize)]
pub struct FTruncate {
    pub fd: i32,
    pub lenght: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ReadDir {
    pub dir_id: i32,
    pub item_type: u32,
    //name: [u8],
}

#[derive(Serialize, Deserialize, Default)]
pub struct Stat {
    pub dev: u32,       /* Device ID of device containing file */
    pub mode: u32,      /* File type, attributes, and access mode bits */
    pub rdev: u32,      /* Device ID (if file is character or block special) */
    pub ino: u16,       /* File serial number */
    pub nlink: u16,     /* Number of hard links to the file */
    pub size: i64,      /* Size of file/directory, in bytes */
    pub atim_sec: i64,  /* Time of last access, seconds */
    pub atim_nsec: i64, /* Time of last access, nanoseconds */
    pub mtim_sec: i64,  /* Time of last modification, seconds */
    pub mtim_nsec: i64, /* Time of last modification, nanoseconds */
    pub ctim_sec: i64,  /* Time of last status change, seconds */
    pub ctim_nsec: i64, /* Time of last status change, nanoseconds */
    pub blocks: u64,    /* Number of blocks allocated */
    pub uid: i16,       /* User ID of file */
    pub gid: i16,       /* Group ID of file */
    pub blksize: i16,   /* Block size used for filesystem I/O */
    pub reserved: u16,  /* Reserved space */
                        // union
                        //  - s32 fd
                        //  - char pathname[]
}

#[derive(Deserialize)]
pub struct Chstat {
    pub stat: Stat,
    pub _flags: i16, /* flags */
                     // union
                     //  - s32 fd
                     //  - char pathname[]
}

#[derive(Serialize)]
pub struct Statfs {
    pub fstype: u32,
    pub reserved: u32,
    pub namelen: i64, // originally u64
    pub bsize: i64,   // originally u64
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    //pathname: [u8],
}

#[derive(Deserialize)]
pub struct MkDir {
    pub mode: u32,
    pub _reserved: u32,
    //pathname: [u8],
}
