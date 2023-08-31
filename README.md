# Thunderbird Updater

Manage a local thunderbird install. Primarilly built since I don't have root access
on LiU's computers and the only mailprogram there is *evolution*.

## Running and building

To compile the program [install the rust toolchain](https://rustup.rs) if you haven't already.
Thereafter simply clone the main branch and run `cargo build --release`. The binary will then be in the `target/release/` directory. For now only Linux is a supported platform, but it might change if someone wants to add support for it or if I'll find the need for it.

## Extending the program & merge requests

Merge requests are encouraged, and I'm open to extend the program's functionallity. I'm open to extend it to support a range of popular programs or simply make it the best at it's job.
