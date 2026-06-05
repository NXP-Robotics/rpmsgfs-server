/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use crate::rpmsgfs::io;
use crate::rpmsgfs::map;
use crate::rpmsgfs::msgs;
use bincode::{deserialize, serialize};
use log::{info, trace};
use nix::libc;
use std::fs;
use std::fs::File;
use std::fs::ReadDir;
use std::io::Error;
use std::io::Seek;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

pub const RESULT_DO_NOT_SEND_RESPONSE: i32 = 0xAAAA;
const MAX_CONTENT_SIZE: usize = 200;

fn str_from_u8_nul_utf8(utf8_src: &[u8]) -> &str {
    let nul_range_end = utf8_src
        .iter()
        .position(|&c| c == b'\0')
        .unwrap_or(utf8_src.len()); // default to length if no `\0` present
    ::std::str::from_utf8(&utf8_src[0..nul_range_end]).unwrap_or("")
}

pub fn normalize_lexically(pathbuf: &PathBuf) -> Result<PathBuf, Error> {
    let mut lexical = PathBuf::new();
    let mut iter = pathbuf.components().peekable();

    // Find the root, if any, and add it to the lexical path.
    // Here we treat the Windows path "C:\" as a single "root" even though
    // `components` splits it into two: (Prefix, RootDir).
    let root = match iter.peek() {
        Some(Component::ParentDir) => return Err(Error::from_raw_os_error(libc::ENOENT)),
        Some(p @ Component::RootDir) | Some(p @ Component::CurDir) => {
            lexical.push(p);
            iter.next();
            lexical.as_os_str().len()
        }
        Some(Component::Prefix(prefix)) => {
            lexical.push(prefix.as_os_str());
            iter.next();
            if let Some(p @ Component::RootDir) = iter.peek() {
                lexical.push(p);
                iter.next();
            }
            lexical.as_os_str().len()
        }
        None => return Ok(PathBuf::new()),
        Some(Component::Normal(_)) => 0,
    };

    for component in iter {
        match component {
            Component::RootDir => unreachable!(),
            Component::Prefix(_) => return Err(Error::from_raw_os_error(libc::ENOENT)),
            Component::CurDir => continue,
            Component::ParentDir => {
                // It's an error if ParentDir causes us to go above the "root".
                if lexical.as_os_str().len() == root {
                    return Err(Error::from_raw_os_error(libc::ENOENT));
                } else {
                    lexical.pop();
                }
            }
            Component::Normal(path) => lexical.push(path),
        }
    }
    Ok(lexical)
}

fn normalize_path(export_path: &String, path: &str) -> Result<String, Error> {
    let mut first = export_path.clone();
    if !export_path.is_empty() && export_path.chars().last().unwrap() == '/' {
        first.pop();
    }

    let second: String = if !path.is_empty() {
        if path.chars().next().unwrap() == '/' {
            path.to_string()
        } else {
            String::from("/") + path
        }
    } else {
        String::from("")
    };

    let all = first.clone() + second.as_str();

    let pathbuf = Path::new(&all).to_path_buf();

    // TODO: use pathbuf.normalize_lexically once rust issue 134694 is solved
    match normalize_lexically(&pathbuf) {
        Ok(result) => {
            let normalized_path = result.into_os_string().into_string().unwrap();
            if normalized_path.starts_with(&first) {
                Ok(normalized_path)
            } else {
                Err(Error::from_raw_os_error(libc::ENOENT))
            }
        }
        Err(_) => Err(Error::from_raw_os_error(libc::ENOENT)),
    }
}

fn get_path_and_verify(export_path: &String, utf8_src: &[u8]) -> Result<String, Error> {
    let path = str_from_u8_nul_utf8(utf8_src);
    normalize_path(export_path, path)
}

pub fn open(
    files: &mut map::Map<File>,
    export_path: &String,
    data: &[u8],
) -> Result<(i32, Vec<u8>), Error> {
    let open_data: msgs::Open = deserialize(&data).unwrap();

    let path_offset = std::mem::size_of::<msgs::Open>();
    let path = get_path_and_verify(export_path, &data[path_offset..])?;
    info!(
        "open {:?}, mode:{:o}, flags:0x{:x}",
        path, open_data.mode, open_data.flags
    );

    let custom_flags: i32 = match open_data.flags & (msgs::O_WRITE | msgs::O_READ) {
        /*msgs::O_WRITE | msgs::O_READ*/ 3 => libc::O_RDWR,
        msgs::O_WRITE => libc::O_WRONLY,
        _ => libc::O_RDONLY,
    } | match open_data.flags & msgs::O_NOFOLLOW {
        msgs::O_NOFOLLOW => libc::O_NOFOLLOW,
        _ => 0,
    } | match open_data.flags & msgs::O_EXCL {
        msgs::O_EXCL => libc::O_EXCL,
        _ => 0,
    } | match open_data.flags & msgs::O_NONBLOCK {
        msgs::O_NONBLOCK => libc::O_NONBLOCK,
        _ => 0,
    } | match open_data.flags & msgs::O_SYNC {
        msgs::O_SYNC => libc::O_SYNC,
        _ => 0,
    } | match open_data.flags & msgs::O_DIRECT {
        msgs::O_DIRECT => libc::O_DIRECT,
        _ => 0,
    } | match open_data.flags & msgs::O_DIRECTORY {
        msgs::O_DIRECTORY => libc::O_DIRECTORY,
        _ => 0,
    } | match open_data.flags & msgs::O_LARGEFILE {
        msgs::O_LARGEFILE => libc::O_LARGEFILE,
        _ => 0,
    } | match open_data.flags & msgs::O_NOATIME {
        msgs::O_NOATIME => libc::O_NOATIME,
        _ => 0,
    } | match open_data.flags & msgs::O_CREAT {
        msgs::O_CREAT => libc::O_CREAT,
        _ => 0,
    } | match open_data.flags & msgs::O_APPEND {
        msgs::O_APPEND => libc::O_APPEND,
        _ => 0,
    } | match open_data.flags & msgs::O_TRUNC {
        msgs::O_TRUNC => libc::O_TRUNC,
        _ => 0,
    };

    // Note: The std::fs::OpenOptions::create function is not used because it
    //       doesn't allow to open files with O_READ | O_CREAT.
    //       Instead the create flag is passed to
    //       std::os::unix::fs::OpenOptions::custom_flags in which the error
    //       is not triggered.

    let file = std::fs::OpenOptions::new()
        .read((open_data.flags & msgs::O_READ) == msgs::O_READ)
        .write((open_data.flags & msgs::O_WRITE) == msgs::O_WRITE)
        .custom_flags(custom_flags)
        .mode(open_data.mode)
        .open(path.clone())?;

    Ok((files.add(file, path), vec![]))
}

pub fn close(files: &mut map::Map<File>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let fd: i32 = deserialize(&data).unwrap();
    info!("close {:}", fd);

    files.remove(fd)?;
    Ok((0, vec![]))
}

pub fn read(
    files: &mut map::Map<File>,
    rpmsgfs_io: &mut io::Io,
    header: &msgs::Header,
    data: &[u8],
) -> Result<(i32, Vec<u8>), Error> {
    let read_data: msgs::FileContent = deserialize(&data).unwrap();
    info!("read from {:}", read_data.fd);

    let (file, _) = files.get_mut(read_data.fd)?;

    let mut pending_bytes = read_data.content_size as usize;
    while pending_bytes > 0 {
        trace!("pending_bytes = {:}", pending_bytes);
        let mut buf = vec![];
        let max_chunk_size = match pending_bytes < MAX_CONTENT_SIZE {
            true => pending_bytes,
            false => MAX_CONTENT_SIZE,
        };
        let mut chunk = file.take(max_chunk_size as u64);
        trace!("buf len = {:}", buf.len());
        let bytes_read = chunk.read_to_end(&mut buf)?;
        trace!("size = {:}", bytes_read);
        trace!("{:?}", buf);
        let response = [serialize(&read_data).unwrap(), buf].concat();

        pending_bytes = pending_bytes - bytes_read;

        // if no bytes read then end the read process
        if bytes_read == 0 {
            pending_bytes = 0;
        }

        rpmsgfs_io
            .send_response(header, bytes_read as i32, response)
            .expect("cannot send read response");
    }
    Ok((RESULT_DO_NOT_SEND_RESPONSE, vec![]))
}

pub fn write(
    files: &mut map::Map<File>,
    header: &msgs::Header,
    data: &[u8],
) -> Result<(i32, Vec<u8>), Error> {
    let write_data: msgs::FileContent = deserialize(&data).unwrap();

    let content_offset = std::mem::size_of::<msgs::FileContent>();
    let content = &data[content_offset..(content_offset + (write_data.content_size as usize))];

    let (file, _) = files.get_mut(write_data.fd)?;
    info!("write to {:}", &write_data.fd);

    let size = file.write(content)?;

    if header.cookie != 0 {
        Ok((size as i32, vec![]))
    } else {
        Ok((RESULT_DO_NOT_SEND_RESPONSE, vec![]))
    }
}

pub fn seek(files: &mut map::Map<File>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let seek_data: msgs::Seek = deserialize(&data).unwrap();
    info!("seek {:}", seek_data.fd);

    let (file, _) = files.get_mut(seek_data.fd)?;

    let result = file.seek(match seek_data.whence {
        0 => std::io::SeekFrom::Start(seek_data.offset as u64),
        2 => std::io::SeekFrom::End(seek_data.offset as i64),
        _ => std::io::SeekFrom::Current(seek_data.offset as i64),
    })?;
    match result.try_into() {
        Ok(result_offset) => Ok((result_offset, vec![])),
        Err(_) => Err(Error::from_raw_os_error(libc::EFAULT)),
    }
}

pub fn sync(files: &mut map::Map<File>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let fd: i32 = deserialize(&data).unwrap();
    info!("sync {:}", fd);

    let (file, _) = files.get_mut(fd)?;

    file.sync_all()?;
    Ok((0, vec![]))
}

pub fn ftruncate(files: &mut map::Map<File>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let ftruncate_data: msgs::FTruncate = deserialize(&data).unwrap();
    info!("ftruncate {:}", ftruncate_data.fd);

    let (file, _) = files.get_mut(ftruncate_data.fd)?;

    file.set_len(ftruncate_data.lenght as u64)?;
    Ok((0, vec![]))
}

pub fn opendir(
    directories: &mut map::Map<ReadDir>,
    export_path: &String,
    data: &[u8],
) -> Result<(i32, Vec<u8>), Error> {
    let path = get_path_and_verify(export_path, &data)?;
    info!("opendir {:?}", path);

    let dir = fs::read_dir(path.clone())?;
    Ok((directories.add(dir, path), vec![]))
}

fn convert_file_type(dir_entry: &std::fs::DirEntry) -> u32 {
    match dir_entry.file_type() {
        Ok(file_type) => {
            if file_type.is_file() {
                msgs::DT_REG
            } else if file_type.is_char_device() {
                msgs::DT_CHR
            } else if file_type.is_block_device() {
                msgs::DT_BLK
            } else if file_type.is_dir() {
                msgs::DT_DIR
            } else if file_type.is_symlink() {
                msgs::DT_LNK
            } else if file_type.is_fifo() {
                msgs::DT_FIFO
            } else if file_type.is_socket() {
                msgs::DT_SOCK
            } else {
                msgs::DT_UNKNOWN
            }
        }
        Err(_) => msgs::DT_UNKNOWN,
    }
}

pub fn readdir(directories: &mut map::Map<ReadDir>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let readdir_data: msgs::ReadDir = deserialize(&data).unwrap();

    info!("readdir {:}", &readdir_data.dir_id);

    let (dir, _) = directories.get_mut(readdir_data.dir_id)?;
    match dir.next() {
        Some(item) => {
            let dir_entry = item?;
            let readdir_response = msgs::ReadDir {
                dir_id: readdir_data.dir_id,
                item_type: convert_file_type(&dir_entry),
            };
            let filename = dir_entry.file_name().into_vec();
            let response = [serialize(&readdir_response).unwrap(), filename, vec![0]].concat();

            Ok((0, response))
        }
        None => Err(Error::from_raw_os_error(libc::ENOENT)),
    }
}

pub fn rewinddir(
    directories: &mut map::Map<ReadDir>,
    data: &[u8],
) -> Result<(i32, Vec<u8>), Error> {
    let dir_id: i32 = deserialize(&data).unwrap();
    info!("rewinddir {:}", dir_id);

    /* Rewind is not possible so just remove and reopen dir */
    let directory = directories.get_mut(dir_id)?;
    directory.0 = fs::read_dir(&directory.1)?;
    Ok((0, vec![]))
}

pub fn closedir(directories: &mut map::Map<ReadDir>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let dir_id: i32 = deserialize(&data).unwrap();
    info!("closedir {:}", dir_id);

    directories.remove(dir_id)?;
    Ok((0, vec![]))
}

pub fn statfs(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let path_offset = std::mem::size_of::<msgs::Statfs>();
    let path = get_path_and_verify(export_path, &data[path_offset..])?;

    info!("statfs {:?}", path);

    match nix::sys::statfs::statfs(if path.is_empty() { "/" } else { &path }) {
        Ok(statfs) => {
            let statfs_data = msgs::Statfs {
                fstype: u32::try_from(statfs.filesystem_type().0).unwrap_or(0),
                reserved: 0,
                namelen: statfs.maximum_name_length() as i64,
                bsize: statfs.block_size() as i64,
                blocks: statfs.blocks(),
                bfree: statfs.blocks_free(),
                bavail: statfs.blocks_available(),
                files: statfs.files(),
                ffree: statfs.files_free(),
            };
            Ok((0, serialize(&statfs_data).unwrap()))
        }
        Err(e) => Err(Error::from_raw_os_error(e as i32)),
    }
}

fn stat_helper(path: &str) -> Result<(i32, Vec<u8>), Error> {
    let stat_result = nix::sys::stat::stat(path)?;
    let stat_response = msgs::Stat {
        dev: stat_result.st_dev as u32,
        mode: stat_result.st_mode,
        rdev: stat_result.st_rdev as u32,
        ino: stat_result.st_ino as u16,
        nlink: stat_result.st_nlink as u16,
        size: stat_result.st_size as i64,
        atim_sec: stat_result.st_atime as i64,
        atim_nsec: stat_result.st_atime_nsec as i64,
        mtim_sec: stat_result.st_mtime as i64,
        mtim_nsec: stat_result.st_mtime_nsec as i64,
        ctim_sec: stat_result.st_ctime as i64,
        ctim_nsec: stat_result.st_ctime_nsec as i64,
        blocks: stat_result.st_blocks as u64,
        uid: stat_result.st_uid as i16,
        gid: stat_result.st_gid as i16,
        blksize: stat_result.st_blksize as i16,
        reserved: 0,
    };
    Ok((0, serialize(&stat_response).unwrap()))
}

pub fn fstat(files: &mut map::Map<File>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let path_offset = std::mem::size_of::<msgs::Stat>();
    let file_descriptor: i32 = deserialize(&data[path_offset..]).unwrap();

    let (_, path) = files.get_mut(file_descriptor)?;
    info!("fstat {:?}", path);

    stat_helper(path)
}

pub fn stat(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let path_offset = std::mem::size_of::<msgs::Stat>();
    let path = get_path_and_verify(export_path, &data[path_offset..])?;
    info!("stat {:?}", path);

    stat_helper(path.as_str())
}

fn chstat_helper(path: &str, chstat_data: &msgs::Chstat) -> Result<(), Error> {
    let mode = nix::sys::stat::Mode::from_bits(chstat_data.stat.mode)
        .unwrap_or(nix::sys::stat::Mode::empty());
    nix::sys::stat::fchmodat(
        nix::fcntl::AT_FDCWD,
        path,
        mode,
        nix::sys::stat::FchmodatFlags::FollowSymlink,
    )?;

    let atime = nix::sys::time::TimeSpec::new(
        chstat_data.stat.atim_sec as nix::sys::time::time_t,
        chstat_data.stat.atim_nsec as nix::sys::time::time_t,
    );
    let mtime = nix::sys::time::TimeSpec::new(
        chstat_data.stat.mtim_sec as nix::sys::time::time_t,
        chstat_data.stat.mtim_nsec as nix::sys::time::time_t,
    );
    nix::sys::stat::utimensat(
        nix::fcntl::AT_FDCWD,
        path,
        &atime,
        &mtime,
        nix::sys::stat::UtimensatFlags::FollowSymlink,
    )?;
    Ok(())
}

pub fn fchstat(files: &mut map::Map<File>, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let chstat_data: msgs::Chstat = deserialize(&data).unwrap();
    let path_offset = std::mem::size_of::<msgs::Chstat>();
    let file_descriptor: i32 = deserialize(&data[path_offset..]).unwrap();
    info!("fchstat {:}", file_descriptor);

    let (_, path) = files.get_mut(file_descriptor)?;
    chstat_helper(path, &chstat_data)?;
    Ok((0, vec![]))
}

pub fn chstat(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let chstat_data: msgs::Chstat = deserialize(&data).unwrap();
    let path_offset = std::mem::size_of::<msgs::Chstat>();
    let path = get_path_and_verify(export_path, &data[path_offset..])?;
    info!("chstat {:?}", path);

    chstat_helper(path.as_str(), &chstat_data)?;
    Ok((0, vec![]))
}

pub fn unlink(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let path = get_path_and_verify(export_path, &data)?;
    info!("unlink {:?}", path);

    fs::remove_file(path)?;
    Ok((0, vec![]))
}

pub fn mkdir(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let mkdir_data: msgs::MkDir = deserialize(&data).unwrap();
    let path_offset = std::mem::size_of::<msgs::MkDir>();
    let path = get_path_and_verify(export_path, &data[path_offset..])?;
    info!("mkdir {:?}", path);

    std::fs::DirBuilder::new()
        .mode(mkdir_data.mode)
        .create(path)?;
    Ok((0, vec![]))
}

pub fn rmdir(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let path = get_path_and_verify(export_path, &data)?;
    info!("rmdir {:?}", path);

    fs::remove_dir(path)?;
    Ok((0, vec![]))
}

pub fn rename(export_path: &String, data: &[u8]) -> Result<(i32, Vec<u8>), Error> {
    let path_from = get_path_and_verify(export_path, &data)?;
    let path_to_offset = (path_from.len() + 1 + 0x7) & !0x7;
    let path_to = get_path_and_verify(export_path, &data[path_to_offset..])?;
    info!("rename {:?}->{:?}", path_from, path_to);

    fs::rename(path_from, path_to)?;
    Ok((0, vec![]))
}

#[cfg(test)]
mod test_commands {
    use crate::rpmsgfs::commands;
    use crate::rpmsgfs::commands::*;
    use parameterized::parameterized;
    use serde_derive::Serialize;

    #[parameterized(data = {
        ("/tmp", "/tmp", "/"),
        ("/tmp", "/tmp", ""),
        ("/tmp", "", "tmp"),
        ("/tmp/test", "/tmp/", "/test"),
        ("/tmp/test", "/tmp", "test"),
        ("/tmp/test", "/tmp", "just_a_dir/just_another_dir/../../just_a_dir/../test"),
    })]
    fn test_normalize_path(data: (&str, &str, &str)) {
        let (expected, export_path, path) = data;
        assert_eq!(
            expected,
            normalize_path(&String::from(export_path), path).unwrap()
        );
    }

    #[parameterized(data = {
        ("/tmp/just_a_dir", "../may_not_be_seen"),
        ("/tmp/just_a_dir", "just_another_dir/../../may_not_be_seen"),
    })]
    fn test_normalize_path_should_fail(data: (&str, &str)) {
        let (export_path, path) = data;
        assert_eq!(
            libc::ENOENT,
            normalize_path(&String::from(export_path), path)
                .unwrap_err()
                .raw_os_error()
                .unwrap()
        );
    }

    #[derive(Serialize)]
    pub struct Open {
        pub flags: i32,
        pub mode: u32,
    }

    fn open(path: String, flags: i32, files: &mut map::Map<File>) -> Result<(i32, Vec<u8>), Error> {
        let open_data = serialize(&Open {
            flags: flags,
            mode: 0o644,
        })
        .unwrap();
        let binding = [open_data, path.as_bytes().to_vec()].concat();
        let combined = binding.as_slice();
        commands::open(files, &"/tmp".to_string(), &combined)
    }

    fn write(fd: i32, data: &[u8], files: &mut map::Map<File>) -> Result<(i32, Vec<u8>), Error> {
        let write_data = serialize(&msgs::FileContent {
            fd: fd,
            content_size: data.len().try_into().unwrap(),
        })
        .unwrap();
        let binding = [write_data, data.to_vec()].concat();
        let combined = binding.as_slice();
        commands::write(
            files,
            &msgs::Header {
                command: msgs::CMD_WRITE,
                result: 0,
                cookie: 0,
            },
            &combined,
        )
    }

    #[test]
    fn test_open() {
        let mut files: map::Map<File> = map::Map::new();

        let open_result = open(
            "/blieb".to_string(),
            msgs::O_CREAT | msgs::O_WRITE,
            &mut files,
        )
        .unwrap();
        assert_eq!(open_result.0 >= 0, true);
    }

    #[test]
    fn test_open_file_with_only_read_and_create_flags() {
        let mut files: map::Map<File> = map::Map::new();

        let open_result = open(
            "/opened_with_read_and_create_flags".to_string(),
            msgs::O_CREAT | msgs::O_READ,
            &mut files,
        )
        .unwrap();
        assert_eq!(open_result.0 >= 0, true);

        let write_result = write(open_result.0, "test".as_bytes(), &mut files);
        assert_eq!(write_result.unwrap_err().raw_os_error(), Some(libc::EBADF));
    }

    #[test]
    fn test_open_file_existing_file() {
        let mut files: map::Map<File> = map::Map::new();

        let open_result = open(
            "/opened_with_read_and_create_flags".to_string(),
            msgs::O_APPEND | msgs::O_WRITE | msgs::O_TRUNC,
            &mut files,
        )
        .unwrap();
        assert_eq!(open_result.0 >= 0, true);

        let write_result = write(open_result.0, "test".as_bytes(), &mut files).unwrap();
        assert_eq!(write_result.0 >= 0, true);
    }

    #[test]
    fn test_open_fails_when_reading_not_existing_file() {
        let mut files: map::Map<File> = map::Map::new();

        let _ = fs::remove_file("/tmp/blieb");
        let open_result = open("/blieb".to_string(), msgs::O_READ, &mut files);
        assert_eq!(open_result.unwrap_err().raw_os_error(), Some(libc::ENOENT));
    }

    fn opendir(path: String, directories: &mut map::Map<ReadDir>) -> (i32, Vec<u8>) {
        commands::opendir(directories, &"/tmp".to_string(), path.as_bytes()).unwrap()
    }

    fn readdir(dir_id: i32, directories: &mut map::Map<ReadDir>) -> (i32, Vec<u8>) {
        let readdir_data = serialize(&msgs::ReadDir {
            dir_id: dir_id,
            item_type: 0,
        })
        .unwrap();
        commands::readdir(directories, readdir_data.as_slice()).unwrap()
    }

    fn rewinddir(dir_id: i32, directories: &mut map::Map<ReadDir>) -> (i32, Vec<u8>) {
        let rewinddir_data = serialize(&dir_id).unwrap();
        commands::rewinddir(directories, rewinddir_data.as_slice()).unwrap()
    }

    #[test]
    fn test_rewinddir() {
        let mut directories: map::Map<ReadDir> = map::Map::new();
        let opendir_result = opendir("/".to_string(), &mut directories);
        assert_eq!(opendir_result.0 >= 0, true);

        let first_readdir_result = readdir(opendir_result.0, &mut directories);
        assert_eq!(first_readdir_result.0, 0);

        let rewinddir_result = rewinddir(opendir_result.0, &mut directories);
        assert_eq!(rewinddir_result.0, 0);

        // Do a read again and it should return the same name
        let second_readdir_result = readdir(opendir_result.0, &mut directories);
        assert_eq!(second_readdir_result.0, 0);
        assert_eq!(first_readdir_result.1, second_readdir_result.1);
    }
}
