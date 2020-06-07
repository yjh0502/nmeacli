## nmeacli

nmeacli is a TUI-based NMEA data stream visualizer. [demo](https://asciinema.org/a/IoL43H9WCeLREtKQgiUl5T5G6)

```
# read from serial device
NMEACLI_DEV=/dev/TTYACM0 cargo run --bin nmeacli

# or, read from TCP
NMEACLI_ADDR=127.0.0.1:10021 cargo run --bin nmeacli
```
