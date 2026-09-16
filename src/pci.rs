use crate::config::PciFormat;
use crate::error::IsoTpError;

/// Size at which a single-frame message must be sent using FD PCI
const SF_FD_THRESHOLD: usize = 8;

/// Size at which a multi-frame message must be sent using FD PCI
const FF_FD_THRESHOLD: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowStatus {
    ContinueToSend,
    Wait,
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pci {
    SingleFrame { header_len: usize, len: usize },
    FirstFrame { header_len: usize, total_len: usize },
    ConsecutiveFrame { seq: u8 },
    FlowControl { status: FlowStatus, block_size: u8, st_min: u8 },
}

/// Parses the N_PCI of an ISO-TP frame, validating the encoding against `rx_format`.
pub fn parse(data: &[u8], rx_format: PciFormat) -> Result<Pci, IsoTpError> {
    let first = *data.first().ok_or(IsoTpError::PciInvalid)?;
    match first >> 4 {
        0x0 => {
            let nibble_len = (first & 0x0F) as usize;
            if nibble_len == 0 {
                if rx_format == PciFormat::Classic {
                    return Err(IsoTpError::PciFormatNotConfigured);
                }
                let len = *data.get(1).ok_or(IsoTpError::PciInvalid)? as usize;
                Ok(Pci::SingleFrame { header_len: 2, len })
            } else {
                if rx_format == PciFormat::Fd {
                    return Err(IsoTpError::PciFormatNotConfigured);
                }
                Ok(Pci::SingleFrame { header_len: 1, len: nibble_len })
            }
        }
        0x1 => {
            let msb = (first & 0x0F) as usize;
            let lsb = *data.get(1).ok_or(IsoTpError::PciInvalid)? as usize;
            let classic_len = (msb << 8) | lsb;
            if classic_len == 0 {
                if rx_format == PciFormat::Classic {
                    return Err(IsoTpError::PciFormatNotConfigured);
                }
                if data.len() < 6 {
                    return Err(IsoTpError::PciInvalid);
                }
                let total_len = ((data[2] as usize) << 24)
                    | ((data[3] as usize) << 16)
                    | ((data[4] as usize) << 8)
                    | data[5] as usize;
                Ok(Pci::FirstFrame { header_len: 6, total_len })
            } else {
                if rx_format == PciFormat::Fd {
                    return Err(IsoTpError::PciFormatNotConfigured);
                }
                Ok(Pci::FirstFrame { header_len: 2, total_len: classic_len })
            }
        }
        0x2 => Ok(Pci::ConsecutiveFrame { seq: first & 0x0F }),
        0x3 => {
            let status = match first & 0x0F {
                0x0 => FlowStatus::ContinueToSend,
                0x1 => FlowStatus::Wait,
                _ => FlowStatus::Overflow,
            };
            let block_size = *data.get(1).ok_or(IsoTpError::PciInvalid)?;
            let st_min = *data.get(2).ok_or(IsoTpError::PciInvalid)?;
            Ok(Pci::FlowControl { status, block_size, st_min })
        }
        _ => Err(IsoTpError::PciInvalid),
    }
}

/// Whether a Single Frame of `len` bytes should use the escaped (FD) header, per `format`.
/// `Err` means `format` can't represent `len` as a single frame at all (falls back to First Frame).
pub fn use_escaped_single(format: PciFormat, len: usize) -> Result<bool, IsoTpError> {
    match format {
        PciFormat::Classic if len >= SF_FD_THRESHOLD => Err(IsoTpError::MessageTooLarge),
        PciFormat::Classic => Ok(false),
        PciFormat::Fd => Ok(true),
        PciFormat::Auto => Ok(len >= SF_FD_THRESHOLD),
    }
}

/// Whether a First Frame for a `total_len`-byte message should use the escaped (FD) header.
/// `Err` means `format` can't represent `total_len` at all (Classic's field maxes at 4095).
pub fn use_escaped_first(format: PciFormat, total_len: usize) -> Result<bool, IsoTpError> {
    match format {
        PciFormat::Classic if total_len >= FF_FD_THRESHOLD => Err(IsoTpError::MessageTooLarge),
        PciFormat::Classic => Ok(false),
        PciFormat::Fd => Ok(true),
        PciFormat::Auto => Ok(total_len >= FF_FD_THRESHOLD),
    }
}

pub fn encode_single_header(buf: &mut [u8], len: usize, escaped: bool) -> usize {
    if escaped {
        buf[0] = 0x00;
        buf[1] = len as u8;
        2
    } else {
        buf[0] = len as u8;
        1
    }
}

pub fn encode_first_header(buf: &mut [u8], total_len: usize, escaped: bool) -> usize {
    if escaped {
        buf[0] = 0x10;
        buf[1] = 0x00;
        buf[2] = (total_len >> 24) as u8;
        buf[3] = (total_len >> 16) as u8;
        buf[4] = (total_len >> 8) as u8;
        buf[5] = total_len as u8;
        6
    } else {
        buf[0] = 0x10 | ((total_len >> 8) as u8 & 0x0F);
        buf[1] = total_len as u8;
        2
    }
}

pub fn encode_consecutive_header(buf: &mut [u8], seq: u8) -> usize {
    buf[0] = 0x20 | (seq & 0x0F);
    1
}

pub fn encode_flow_control(buf: &mut [u8], status: FlowStatus, block_size: u8, st_min: u8) -> usize {
    let status_bits = match status {
        FlowStatus::ContinueToSend => 0x0,
        FlowStatus::Wait => 0x1,
        FlowStatus::Overflow => 0x2,
    };
    buf[0] = 0x30 | status_bits;
    buf[1] = block_size;
    buf[2] = st_min;
    3
}

/// Sequence numbers on Consecutive Frames start at 1 and wrap 0..=15.
pub fn next_seq(seq: u8) -> u8 {
    if seq == 0x0F { 0 } else { seq + 1 }
}

/// Decodes a flow-control separation-time byte into microseconds.
pub fn st_min_to_us(byte: u8) -> u32 {
    match byte {
        0x00..=0x7F => byte as u32 * 1000,
        0xF1..=0xF9 => (byte - 0xF0) as u32 * 100,
        _ => 0,
    }
}

/// Encodes a microsecond separation time into a flow-control separation-time byte. Values over
/// 127ms clamp to the maximum (0x7F = 127ms).
pub fn us_to_st_min(us: u32) -> u8 {
    match us {
        0..100 => 0x00,
        100..1000 => (us / 100) as u8 + 0xF0,
        1000..=127_000 => (us / 1000) as u8,
        _ => 0x7F,
    }
}
