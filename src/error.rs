/// Errors that can occur while running the ISO-TP transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoTpError {
    /// Message is too large for the buffer
    MessageTooLarge,
    /// A received frame's PCI (protocol control information) byte was malformed, or not valid
    /// for the transport's current state.
    InvalidPci,
    /// A consecutive frame arrived with an unexpected sequence number.
    UnexpectedSequenceNumber,
    /// Reassembly finished (or a single-frame message arrived) while the previously completed
    /// message was still sitting unread in `pending_rx`. The new message is rejected rather
    /// than silently overwriting the one the caller hasn't taken via `read()` yet.
    RxNotDrained,
    /// A timeout (N_Bs/N_Cr/etc.) elapsed while waiting for the next frame.
    Timeout,
}
