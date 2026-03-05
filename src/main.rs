/*
 * Copyright 2026 NXP
 * All rights reserved.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 *
 */

use std::path::Path;
mod rpmsgfs;
use crate::rpmsgfs::Rpmsgfs;

fn main() {
    std_logger::Config::logfmt().init();
    let argument = std::env::args()
        .nth(1)
        .expect("No rpmsg device filename given");
    let rpmsg_path = Path::new(&argument);

    let mut rpmsgfs = Rpmsgfs::new(&rpmsg_path);

    loop {
        rpmsgfs.process_command()
    }
}
