use std::io::Write;

pub fn main() -> std::io::Result<()> {
    let rst = ublox::CfgRstBuilder {
        nav_bbr_mask: ublox::NavBbrMask::all(),
        reset_mode: ublox::ResetMode::HardwareResetImmediately,
        reserved1: 0,
    };
    let bytes = rst.into_packet_bytes();

    let mut file = std::fs::File::create("msg.bin")?;
    file.write(&bytes[..])?;

    Ok(())
}
