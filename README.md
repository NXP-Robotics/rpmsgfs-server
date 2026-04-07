# rpmsgfs-server

rpmsgfs-server is an **RPMsg File System server** implementation written in **Rust** and compatible with [NuttX RPMsgFS](https://nuttx.apache.org/docs/latest/components/filesystem/rpmsgfs.html).
It is intended to run on the *master / host processor* in a heterogeneous multi-core system and export a local file system over **RPMsg** to a remote processor.

This repository provides the server-side component of an RPMsg-based file system, enabling a remote core (for example running NuttX) to mount and access files that physically reside on the server core.

## Features

- RPMsg-based file system server
- Written in **Rust**
- Designed for heterogeneous SoCs (e.g. Cortex-A ↔ Cortex-M)
- Compatible with [NuttX RPMsgFS](https://nuttx.apache.org/docs/latest/components/filesystem/rpmsgfs.html) client

## Background

RPMsgFS allows a remote core to mount a directory from another core as if it were a local file system.  
Typical use cases include:
- Debugging and inspection of remote file systems
- Sharing configuration, logs, or runtime data
- Simplifying multi-core software architectures

On the **server side**, a process exposes a directory tree via RPMsg.
On the **client side**, an RPMsgFS driver mounts that directory into the local VFS.

This project implements the **server side** of that architecture. 

## Requirements

- Rust toolchain (stable)
- Linux kernel with RPMsg and RPMsg char device support enabled
- Tooling to export the RPMsg channel through IOCTL (eg. rpmsgexport)
- NuttX OS or an RPMsgFS-compatible client on the remote core

## Building

Clone the repository and build using Cargo:

```bash
git clone https://github.com/NXP-Robotics/rpmsgfs-server.git
cd rpmsgfs-server
cargo build --release
```

## Running

The server is intended to be launched on the CPU responsible for hosting the exported file system.

A typical workflow is:

 1. Start RPMsg infrastructure on both processors
 2. Mount the exported directory on the remote core.
    An example for NuttX would be:
    ```bash
    mount -t rpmsgfs -o cpu=netcore,fs=/shared /mnt/rpmsg
    ```
 3. On the host core, the RPMsg channel need to be exported through IOCTL. An example with [rpmsgexport](https://github.com/andersson/rpmsgexport) is:
    ```bash
    rpmsgexport /dev/rpmsg_ctrl0 rpmsgfs 1024 1025
    ```
 4. Launch rpmsgfs-server on the host:
    ```bash
    ./rpmsgfs-server /dev/rpmsg0
    ```





