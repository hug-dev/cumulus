# cumulus ☁️

https://hug-dev.github.io/cumulus/

An online First-Person View point cloud vizualizer!
The points are colored with the different fields of the point cloud file.

This project was originally developed and authored at **Einride**.

## Supported formats

* [PCD file](https://pointclouds.org/documentation/tutorials/pcd_file_format.html)
* [PLY file](https://en.wikipedia.org/wiki/PLY_(file_format))
* [CSV file](https://en.wikipedia.org/wiki/Comma-separated_values): only 32 bits float values supported

Feel free to contribute adding support for more types!

## Usage

In the browser you are presented with a default point cloud but you can open your own with the dialog.
Very large point clouds might take some time to load and your browser might freeze meanwhile. Small pointclouds should
be instantly loaded.

## Running the CLI locally

Tested on Ubuntu only.

[Install Rust](https://www.rust-lang.org/tools/install) if not already done 🦀.

```
sudo apt-get install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0
cd cumulus
cargo run --release
```

You can even pass a point cloud to the executable to use it directly:

```
./target/release/cumulus resources/pcd_test/example.pcd
```

## Running the Web Assembly version locally

To run the WASM version locally, you do the following:

```
cargo install wasm-bindgen-cli
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --no-typescript --target web --out-dir ./dist/ --out-name "cumulus" ./target/wasm32-unknown-unknown/release/cumulus.wasm
cd dist/
python3 -m http.server
```

And then open http://0.0.0.0:8000/ on a browser!

## TODO

- [ ] implement 2D image with panning and zooming
- [ ] open a file directly in the clipboard
- [ ] do not re-compute row and column if available
- [ ] optimize the binary for size
- [ ] make point picking and colour change faster
- [ ] provide binary as release for CLI
- [ ] add a tutorial point cloud at start

## Contributing

Please feel free to open issues or pull requests to improve this repository! I will be happy to review them :)
