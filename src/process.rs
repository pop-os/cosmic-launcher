// SPDX-License-Identifier: MPL-2.0

use std::process::{exit, Command, Stdio};

use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};

/// Performs a double fork with setsid to spawn and detach a command.
pub fn spawn(mut command: Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        match fork() {
            Ok(ForkResult::Parent { child }) => {
                let _res = waitpid(Some(child), None);
            }

            Ok(ForkResult::Child) => {
                let _res = nix::unistd::setsid();
                let _res = command.spawn();

                exit(0);
            }

            Err(why) => {
                tracing::warn!("failed to fork and spawn command: {}", why.desc());
            }
        }
    }
}
