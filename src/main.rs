/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use clap::Parser;
mod rpmsgfs;
use crate::rpmsgfs::Rpmsgfs;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Export path
    #[arg(short, long, default_value_t = String::from("/"))]
    export_path: String,

    /// RPMsg device
    #[arg(required = true)]
    rpmsg_device: String,
}

fn main() {
    let args = Args::parse();

    std_logger::Config::logfmt().init();

    let mut rpmsgfs = Rpmsgfs::new(args.rpmsg_device, args.export_path);

    loop {
        rpmsgfs.process_command()
    }
}
