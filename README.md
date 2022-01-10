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
Once the server is started, it will parse the metamath database. Wait until it displays the "Ready" message: it shall be a few seconds. You can then switch to a browser and visit [this URL](http://localhost:3030/mmset.raw.html) or for example [this URL](http://localhost:3030/o2p2e4) and start navigating. The port 3030 is not configurable for the moment, but that would be very easily done. Not all pages are served correctly.

### Stopping the server
Just hit CTRL+C to stop the server once you're done browsing!

## Libraries used

* [metamath-knife](https://github.com/david-a-wheeler/metamath-knife) for parsing metamath file and obtaining proofs,
* [handlebars](https://github.com/sunng87/handlebars-rust) for templating,
* [warp](https://github.com/seanmonstar/warp) for the web server.
