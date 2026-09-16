/// Errors that can occur while running the ISO-TP transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoTpError {
    /// Message is too large for the buffer
    MessageTooLarge,
    
    /// A frame's PCI (protocol control information) bytes were malformed — too short for the
    /// frame type they claim to be, or an unrecognized frame type entirely.
    PciInvalid,

    /// A frame's PCI used an encoding `rx_pci_format` doesn't accept — e.g. an FD-escaped header
    /// while configured `Classic`.
    PciFormatNotConfigured,

    /// A structurally valid PCI arrived somewhere it can't be handled right now — e.g. a flow
    /// control frame while idle, or a consecutive frame while not receiving.
    PciUnexpected,

    /// A consecutive frame arrived with an unexpected sequence number.
    UnexpectedSequenceNumber,

    /// A Single Frame or First Frame arrived while the previously completed message was still
    /// sitting unread in `pending_rx`. Rejected outright, before the RX buffer is touched at
    /// all, rather than let a new reception clobber a message the caller hasn't `read()` yet.
    RxNotDrained,

    /// The partner sent a flow-control overflow/abort while we were transmitting.
    PartnerAborted,
    
    /// A timeout (N_Bs/N_Cr/etc.) elapsed while waiting for the next frame.
    Timeout,
}
