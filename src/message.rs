use commkit::{ByteTransfer, Direction, Message};

/// A complete ISO-TP message (already reassembled, or about to be segmented), backed by a
/// fixed-capacity buffer.
///
/// `N` is chosen by the transport that produces or consumes it — see `IsoTpTransport`'s
/// `RX_CAP` parameter — so message sizing stays a compile-time, no-alloc decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoTpMessage<const N: usize> {
    direction: Direction,
    data: [u8; N],
    len: usize,
}

impl<const N: usize> IsoTpMessage<N> {
    /// Builds a message from a data slice. Slices longer than `N` are truncated.
    pub fn new(direction: Direction, data: &[u8]) -> Self {
        let len = core::cmp::min(data.len(), N);
        let mut buf = [0u8; N];
        buf[..len].copy_from_slice(&data[..len]);

        Self { direction, data: buf, len }
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

impl<const N: usize> ByteTransfer for IsoTpMessage<N> {
    fn new(direction: Direction, bytes: &[u8]) -> Self {
        Self::new(direction, bytes)
    }

    fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

impl<const N: usize> Message for IsoTpMessage<N> {}
