use commkit::Transport;
use commkit_can::CanFrame;

use crate::{IsoTpError, IsoTpMessage, IsoTpState};

/// An ISO 15765-2 (ISO-TP) transport for a single logical connection.
///
/// Addressing is entirely the caller's responsibility: only frames already identified as coming
/// from the partner should be handed to `packet_rx`, and whatever frame `packet_tx` returns has
/// no meaningful CAN ID set — the caller stamps the real TX ID onto it before putting it on the
/// wire.
///
/// `TX_CAP`/`RX_CAP` size the fixed outbound/inbound working buffers, and `Inst` is whatever
/// timestamp type the caller's clock produces (only `Copy` is required, so this works with a
/// plain tick counter as well as e.g. an HAL-provided `Instant`).
///
/// TODO: this is a bare shell — `packet_rx`/`packet_tx`/`write`/`read`/`poll` still need the
/// actual segmentation/reassembly/flow-control state machine.
pub struct IsoTpTransport<Inst, const TX_CAP: usize, const RX_CAP: usize> {
    tx_buf: [u8; TX_CAP],
    rx_buf: [u8; RX_CAP],
    /// The last message reassembly (or a single-frame reception) completed, held here until the
    /// caller takes it via `read()`. Single slot, by design: `packet_rx` must reject a newly
    /// completed message with `IsoTpError::RxNotDrained` rather than overwrite this while it's
    /// still `Some` — see the crate-level buffer-ownership discussion.
    pending_rx: Option<IsoTpMessage<RX_CAP>>,
    _instant: core::marker::PhantomData<Inst>,
}

impl<Inst, const TX_CAP: usize, const RX_CAP: usize> IsoTpTransport<Inst, TX_CAP, RX_CAP> {
    pub fn new() -> Self {
        Self {
            tx_buf: [0u8; TX_CAP],
            rx_buf: [0u8; RX_CAP],
            pending_rx: None,
            _instant: core::marker::PhantomData,
        }
    }
}

impl<Inst, const TX_CAP: usize, const RX_CAP: usize> Default
    for IsoTpTransport<Inst, TX_CAP, RX_CAP>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Inst: Copy, const TX_CAP: usize, const RX_CAP: usize> Transport
    for IsoTpTransport<Inst, TX_CAP, RX_CAP>
{
    type Packet = CanFrame;
    type Message = IsoTpMessage<RX_CAP>;
    type Error = IsoTpError;
    type Instant = Inst;
    type State = IsoTpState;

    /// TODO: reassembly/flow-control logic. Once a message is complete (whether from a single
    /// frame or the last consecutive frame), it must be stored via `self.pending_rx = Some(msg)`
    /// — but only after checking `self.pending_rx.is_none()`; if it's already `Some`, return
    /// `Err(IsoTpError::RxNotDrained)` instead of overwriting the undelivered message.
    fn packet_rx(&mut self, _packet: Self::Packet) -> Result<(), Self::Error> {
        todo!()
    }

    fn packet_tx(&mut self) -> Option<Self::Packet> {
        todo!()
    }

    fn write(&mut self, _message: Self::Message) -> Result<(), Self::Error> {
        todo!()
    }

    fn read(&mut self) -> Option<Self::Message> {
        self.pending_rx.take()
    }

    fn poll(&mut self, _now: Self::Instant) -> Self::State {
        todo!()
    }
}
