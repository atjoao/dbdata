# Information

This project provides a Rust implementation of a DLL (`dbdata.dll`) that emulates someones token interface. It is intended for research, reverse engineering, and interoperability purposes. Use responsibly and respect software licenses.

## Compiling

To compile this project you need

- Rust + Cargo + MSVC Compiper

then compile with

```bash
cargo build --release
```

## Usage

This project is to be used with [Uplay R1](https://github.com/atjoao/uplay_r1)

You need to replace the `dbdata.dll` file in the game directory with the one from this project.

Make sure to have a valid account and replace in `uplay.ini` (this file is generated on gamerun) with a valid ownership/license to the game.

## Credits

- [ubi-dbdata](https://github.com/denuvosanctuary/ubi-dbdata/tree/main) for original implementation and fork.
- [UplayDB](https://github.com/UplayDB) for reverse engineering, code and protobuf files
- Claude for writing code