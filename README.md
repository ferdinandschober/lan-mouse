# Lan Mouse Share
Goal of this project is to be an open-source replacement for tools like [Synergy](https://symless.com/synergy) or [Share Mouse](https://www.sharemouse.com/de/).
Currently only wayland is supported but I will take a look at xorg, windows & MacOS in the future.

## Very much unstable
The protocols used for the virtual mouse and virtual keyboard drivers are currently unstable and only supported by wlroots:
- [zwlr\_virtual\_pointer\_manager\_v1](wlr-virtual-pointer-unstable-v1)
- [virtual-keyboard-unstable-v1](https://wayland.app/protocols/virtual-keyboard-unstable-v1)

## Wayland compositor support
|  Required Protocols  (Event Emitting)  | Sway               | Kwin                 | Gnome                |
|----------------------------------------|--------------------|----------------------|----------------------|
| pointer-constraints-unstable-v1        | :heavy_check_mark: | :heavy_check_mark:   | :heavy_check_mark:   |
| relative-pointer-unstable-v1           | :heavy_check_mark: | :heavy_check_mark:   | :heavy_check_mark:   |
| keyboard-shortcuts-inhibit-unstable-v1 | :heavy_check_mark: | :heavy_check_mark:   | :heavy_check_mark:   |
| wlr-layer-shell-unstable-v1            | :heavy_check_mark: | :heavy_check_mark:   | :x:                  |

|  Required Protocols  (Event Receiving) | Sway               | Kwin                 | Gnome                |
|----------------------------------------|--------------------|----------------------|----------------------|
| wlr-virtual-pointer-unstable-v1        | :heavy_check_mark: | :x:                  | :x:                  |
| virtual-keyboard-unstable-v1           | :heavy_check_mark: | :x:                  | :x:                  |
| fake-input                             | :x:                | :heavy_check_mark:   | :x:                  |



Also the [wlr_layer_shell protocol](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) is currently not available on Gnome and may very well [never be](https://gitlab.gnome.org/GNOME/gnome-shell/-/issues/1141) so Gnome support probably requires some sort of Gome-Shell-Extension.

~In order for layershell surfaces to be able to lock the pointer using the pointer\_constraints protocol [this patch](https://github.com/swaywm/sway/pull/7178) needs to be applied to sway.~

## Build and run
First configure the client / server in `config.toml`.
Currently a client is hardcoded to be `client.right`, while a server is configured as `client.left`.
(I know, I know ... )

Client and Server can at the current state not be run on the same server, unless the port is changed in the config in between.

Run Server (sending key events):
```sh
cargo run --bin server
```

Run Client (receiving key events):
```sh
cargo run --bin client
```

As mentioned the server will only work on sway compiled from source with the above mentioned patch applied.

## TODO
- [x] Capture the actual mouse events on the server side via a wayland client and send them to the client
- [x] Mouse grabbing
- [x] Window with absolute position -> wlr\_layer\_shell
- [x] DNS resolving
- [x] Keyboard support
- [x] Scrollwheel support
- [x] Button support
- [ ] Latency measurement + logging
- [ ] Bandwidth usage approximation + logging
- [ ] Multiple IP addresses -> check which one is reachable
- [ ] Merge server and client -> Both client and server can send and receive events depending on what mouse is used where
- [ ] Liveness tracking (automatically ungrab mouse when client unreachable)
- [ ] Clipboard support
- [ ] Graphical frontend (gtk?)
- [ ] *Encrytion* -> likely DTLS
- [ ] Gnome Shell Extension (layer shell is not supported)

## Protocol considerations
Currently *all* mouse and keyboard events are sent via **UDP** for performance reasons.
Each event is sent as one single datagram so in case a packet is lost the event will simly be discarded, which is likely not much of a concern.
**UDP** also has the additional benefit that no reconnection logic is required.
So any client can just go offline and it will simply start working again as soon as it comes back online.

Additionally all server instances (in the future everything will be a server) host a tcp server where critical data, that needs to be send reliably (e.g. the keymap from the server or clipboard contents in the future) can be requested via a tcp connection.
For each request a new connection is established so clients can simply retry if a connection is interrupted.

## Bandwidth considerations
The most bandwidth is taken up by mouse events. A typical office mouse has a polling rate of 125Hz
while gaming mice typically have a much higher polling rate of 1000Hz.
A mouse Event consists of 21 Bytes:
- 1 Byte for the event type enum,
- 4 Bytes (u32) for the timestamp,
- 8 Bytes (f64) for dx,
- 8 Bytes (f64) for dy.

Additionally the IP header with 20 Bytes and the udp header with 8 Bytes take up another 28 Byte.
So in total there is 49 * 1000 Bytes/s for a 1000Hz gaming mouse.
This makes for a bandwidth requirement of 392 kbit/s in total _even_ for a high end gaming mouse.
So bandwidth is a non-issue.

Larger data chunks, like the keymap are offered by the server via tcp listening on the same port.
This way we dont need to implement any congestion control and leave this up to tcp.
In the future this can be used for e.g. clipboard contents as well.


## Security
Sending key and mouse event data over the local network might not be the biggest security concern but in any public network or business environment it's *QUITE* a problem to basically broadcast your keystrokes.
- There should probably be an encryption layer using DTLS below the application to enable a secure link
- The keys could be generated via the graphical frontend
