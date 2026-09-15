use commkit::{ByteTransfer, Message};

/// A complete ISO-TP message (already reassembled, or about to be segmented), backed by a
/// fixed-capacity buffer.
///
/// `N` is chosen by the transport that produces or consumes it — see `IsoTpTransport`'s
/// `RX_CAP` parameter — so message sizing stays a compile-time, no-alloc decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoTpMessage<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> IsoTpMessage<N> {
    /// Builds a message from a data slice. Slices longer than `N` are truncated.
    pub fn new(data: &[u8]) -> Self {
        let len = core::cmp::min(data.len(), N);
        let mut buf = [0u8; N];
        buf[..len].copy_from_slice(&data[..len]);

        Self { data: buf, len }
    }
}

impl<const N: usize> ByteTransfer for IsoTpMessage<N> {
    fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

impl<const N: usize> Message for IsoTpMessage<N> {}
