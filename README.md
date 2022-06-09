# Keyberon-based keyboard firmware for the Cantor keyboard

This is a keyboard firmware for the [Cantor keyboard](https://github.com/diepala/cantor) based on [keyberon](https://github.com/TeXitoi/keyberon).

## Compiling

Install the rust toolchain

```shell
curl https://sh.rustup.rs -sSf | sh
rustup target add thumbv7em-none-eabihf
rustup component add llvm-tools-preview
cargo install cargo-binutils
```

Compile the firmware

```shell
cargo objcopy --release -- -O binary keyberon.bin
```

## Flashing using DFU

Put the developement board in DFU mode by pushing reset while pushing
boot, and then release boot. Then flash it:
```shell
dfu-util -a 0 --dfuse-address 0x08000000 -D keyberon.bin
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

