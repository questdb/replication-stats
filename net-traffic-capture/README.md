# Localhost Network Traffic Packet Throghput Capture Logging

## Install Dependency

To build and run, you need [libpcap](https://www.tcpdump.org/), i.e. the thing
that WireShark is built on.

On Ubuntu, this would be:

```
sudo apt install libpcap-dev
```

## Build

```
cargo build --release
```

## Run with `sudo`

```
sudo ./target/release/net-traffic-capture --help
```

```
Capture time series for TCP IPv4 traffic to specific ports

Usage: net-traffic-capture [OPTIONS] <PORTS>...

Arguments:
  <PORTS>...  List of destination ports to monitor At least one port must be specified

Options:
  -d, --dir <DIR>              Destination directory for output files [default: data]
  -v, --verbosity <VERBOSITY>  Verbosity level 0: silent 1: print a dot for each packet received 2: print packet details 3: print packet flags and zero-data packets 4: also print sent data [default: 0]
  -h, --help                   Print help
  -V, --version                Print version
```
