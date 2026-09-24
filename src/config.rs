use commkit::Duration;
use commkit_can::Dlc;

/// How a Single/First Frame's PCI is read or written.
///
/// On `tx_pci_format` this is a choice of what to emit. On `rx_pci_format`
/// it's a validation constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciFormat {
    ///  Classic format
    Classic,

    ///  Extended format, classic bytes escaped with 0x00
    Fd,

    ///  RX: Checks escape byte to automatically read PCI based on format
    /// 
    ///  TX: Classic unless message size is long enough to require FD
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoTpConfig {
    /// Byte to use for filling the frame where data wasn't written
    pub padding_byte: u8,

    /// DLC for transmitted frames
    /// 
    /// None = smallest possible DLC
    /// DLC = hardcoded DLC
    pub tx_frame_size: Option<Dlc>,

    /// PCI format to use when transmitting frames
    pub tx_pci_format: PciFormat,

    /// PCI formats to accept when receiving frames
    pub rx_pci_format: PciFormat,

    /// Separation time requested in flow control frames we send
    ///
    /// None = no delay (0x00)
    /// Duration = requested delay, rounded up to the next encodable STmin and clamped to 127ms
    pub rx_desired_separation: Option<Duration>,

    /// Block size requested in flow control frames we send
    ///
    /// None = send all frames (0x00)
    /// Block size = frames per block before another FC is required
    pub rx_desired_block_size: Option<u8>,

    pub n_bs_timeout: Duration,

    pub n_cr_timeout: Duration,
}

impl IsoTpConfig {
    /// Default configuration for Classic CAN transport
    pub fn config_init_can() -> Self {
        Self {
            tx_frame_size: Dlc::new(8),
            padding_byte: 0xCC,
            tx_pci_format: PciFormat::Classic,
            rx_pci_format: PciFormat::Classic,
            rx_desired_separation: None,
            rx_desired_block_size: None,
            n_bs_timeout: Duration::from_ticks(1_000_000),
            n_cr_timeout: Duration::from_ticks(1_000_000),
        }
    }

    /// Default configuration for CAN FD transport
    pub fn config_init_can_fd() -> Self {
        Self {
            tx_frame_size: Dlc::new(64),
            padding_byte: 0xCC,
            tx_pci_format: PciFormat::Auto,
            rx_pci_format: PciFormat::Auto,
            rx_desired_separation: None,
            rx_desired_block_size: None,
            n_bs_timeout: Duration::from_ticks(1_000_000),
            n_cr_timeout: Duration::from_ticks(1_000_000),
        }
    }
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        Self::config_init_can()
    }
}
