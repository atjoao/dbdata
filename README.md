# Information

This project provides a Rust implementation of a DLL (`dbdata.dll`) that emulates somethings token interface. It is intended for research, reverse engineering, and interoperability purposes. Use responsibly and respect software licenses.

## Compiling

To compile this project you need

- Rust + Cargo + MSVC Compiper

then compile with

```bash
cargo build --release
```

## Usage

You need to replace the `dbdata.dll` file in the game directory with the one from this project.

Make sure to have a valid account and replace in `dbdata.ini` (this file is generated at runtime) with a valid ownership/license to the game.

## Configuration example

```
[Uplay]
email=email@example.com
password=superpassword
[token]
token=base64
ownership=base64
[settings]
dlcs=12983,23432,23432
```

## Credits

- [ubi-dbdata](https://github.com/denuvosanctuary/ubi-dbdata/tree/main) for original implementation and fork.
- [UplayDB](https://github.com/UplayDB) for reverse engineering, code and protobuf files
- Claude for writing code

> this project is not related to ubisoft or any product.