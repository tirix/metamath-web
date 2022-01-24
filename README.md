# metamath-web
A simple Metamath web server for Rust, displaying proofs in the most basic way.

## How-to
### Installation
First, [install rust](https://www.rust-lang.org/tools/install) if you don't have it yet on your system.

Unfortunately, as long as `metamath-knife` is not on crate.io, one has to clone both `metamath-knife` and this repository:
```
git clone https://github.com/metamath/set.mm.git –-depth 1
git clone https://github.com/tirix/metamath-web.git
git clone https://github.com/david-a-wheeler/metamath-knife.git
```
### Running the server
The following commands can then be used to launch the server:
```
cd metamath-web
cargo run ../set.mm/set.mm
```
### Viewing the pages
Once the server is started, it will parse the metamath database. Wait until it displays the "Ready" message: it shall be a few seconds. You can then switch to a browser and visit for example [http://localhost:3030/mpeascii/o2p2e4](http://localhost:3030/mpeascii/o2p2e4) or [the table of content](http://localhost:3030/mpeascii/toc) and start navigating. The port 3030 is the default, see usage for configuration of the server address and port.

### Stopping the server
Just hit CTRL+C to stop the server once you're done browsing!

## Features and roadmap

Here are some features implemented, and some which are still lacking:

- [x] support for 3 typesettings:
  - [x] ASCII (`mpeascii`) - this is Metamath "source code"
  - [x] Unicode (`mpeuni`) - this is the symbol-by-symbol typesetting
  - [x] STS (`mpests`) - structured typesetting (`sts` feature needed)
- [x] display axioms and definitions' syntax proof
- [x] links to other theorems in comments
- [x] links to bibliographic references (see command line option `-b`)
- [ ] in-line math in comments
- [x] summary of the theorems (hypotheses and statement) before the proof
- [ ] navigation to next/previous theorem in the database
- [x] navigation between the different typesettings
- [x] table of content
- [ ] distinct variables
- [ ] list of uses

## Additional feature

It is possible to serve pages formatted using structured typesetting, by activating the `sts` feature, and browsing [pages in the `mpests` path](http://localhost:3030/mpests/hgt749d).
```
cargo run --features sts ../set.mm/set.mm
```

## Libraries used

* [metamath-knife](https://github.com/david-a-wheeler/metamath-knife) for parsing metamath file and obtaining proofs,
* [handlebars](https://github.com/sunng87/handlebars-rust) for templating,
* [warp](https://github.com/seanmonstar/warp) for the web server.
