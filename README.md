## nmeacli

nmeacli is a TUI-based NMEA data stream visualizer.

[demo.gif](./static/demo.gif)

```
# read from serial device
NMEACLI_DEV=/dev/TTYACM0 cargo run --bin nmeacli

# or, read from TCP
NMEACLI_ADDR=127.0.0.1:10021 cargo run --bin nmeacli
```
