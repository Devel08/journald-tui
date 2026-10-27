<div align="center">
  <h1><code>journald-tui</code></h1>

  <p>
    <strong>A lightweight TUI for viewing and navigating systemd-journald logs in real time.</strong>
  </p>
  <p>🚧 Work in progress - more to come!</p>
</div>

### Requirements
* A linux distribution with systemd
* Rust (`cargo`,`rustc`)
* `systemd-libs` or whatever your distro calls it

### Compile and run
```
git clone https://www.github.com/Devel08/journald-tui.git
cd journald-tui
cargo run --release
```
You can also install `journald-tui` with the following snippet:
```
git clone https://www.github.com/Devel08/journald-tui.git
cd journald-tui
cargo build --release
cp ./target/release/journald-tui /usr/bin/
```
