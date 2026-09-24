/// `N_Result` from spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoTpError {
    /// `N_TIMEOUT_A`: N_As/N_Ar elapsed — the data link didn't confirm a frame in time.
    TimeoutA,

    /// `N_TIMEOUT_Bs`: N_Bs elapsed while the sender was waiting for a flow control frame.
    TimeoutBs,

    /// `N_TIMEOUT_Cr`: N_Cr elapsed while the receiver was waiting for a consecutive frame.
    TimeoutCr,

    /// `N_WRONG_SN`: a consecutive frame arrived with an unexpected sequence number.
    WrongSn,

    /// `N_INVALID_FS`: a flow control frame carried an unknown flow status value.
    InvalidFs,

    /// `N_UNEXP_PDU`: a structurally valid PCI arrived somewhere it can't be handled right now —
    /// e.g. a flow control frame while idle, or a consecutive frame while not receiving.
    UnexpPdu,

    /// `N_WFT_OVRN`: the partner sent more consecutive FlowControl(Wait) frames than N_WFTmax.
    WftOvrn,

    /// `N_BUFFER_OVFLW`: the partner sent a flow-control overflow/abort while we were transmitting.
    BufferOvflw,

    /// Message is too large for the buffer
    MessageTooLarge,

    /// A frame's PCI (protocol control information) bytes were malformed — too short for the
    /// frame type they claim to be, or an unrecognized frame type entirely.
    PciInvalid,

    /// A frame's PCI used an encoding `rx_pci_format` doesn't accept — e.g. an FD-escaped header
    /// while configured `Classic`.
    PciFormatNotConfigured,

    /// A Single Frame or First Frame arrived while the previously completed message was still
    /// sitting unread
    RxNotDrained,
}
