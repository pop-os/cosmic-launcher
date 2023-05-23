// SPDX-License-Identifier: MPL-2.0

use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use std::process::{exit, Command, Stdio};

pub fn spawn(mut command: Command) {
    unsafe {
        match fork().expect("failed to fork process") {
            ForkResult::Parent { child } => {
                waitpid(Some(child), None).unwrap();
            }

            ForkResult::Child => {
                let _res = nix::unistd::setsid();
                let _res = command
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                exit(0);
            }
        }
    }
}
