use commkit::{ByteTransfer, Transport};
use commkit_can::{CanFrame, CanId, MAX_DATA_LEN};

use crate::config::IsoTpConfig;
use crate::error::IsoTpError;
use crate::message::IsoTpMessage;
use crate::pci::{self, FlowStatus, Pci};
use crate::state::IsoTpState;

const PLACEHOLDER_ID: CanId = CanId::Standard(0);

/// An ISO 15765-2 (ISO-TP) transport for a single logical connection.
pub struct IsoTpTransport<Inst, const TX_CAP: usize, const RX_CAP: usize> {
    ///  Behavior for the ISO-TP transport layer
    pub config: IsoTpConfig,

    ///  Fired when any message is received
    pub on_message_received: Option<fn(&[u8])>,

    ///  Fired whenever any error or unexpected condition is encountered
    pub on_error: Option<fn(IsoTpError)>,

    /// Fired whenever a valid packet is sent or received so the caller can run a watchdog and reset() on inactivity
    pub on_activity: Option<fn()>,

    state: IsoTpState,

    tx_buf: [u8; TX_CAP],
    tx_len: usize,
    tx_sent: usize,
    tx_next_seq: u8,

    /// Frames left to send before the next flow-control frame is required. `None` = unlimited.
    tx_block_remaining: Option<u16>,

    /// Separation time from the partner's last flow-control frame, in microseconds.
    tx_st_min_us: u32,

    rx_buf: [u8; RX_CAP],
    rx_len: usize,
    rx_received: usize,
    rx_next_seq: u8,
    pending_rx: Option<IsoTpMessage<RX_CAP>>,

    pending_control: Option<CanFrame>,

    _instant: core::marker::PhantomData<Inst>,
}

impl<Inst, const TX_CAP: usize, const RX_CAP: usize> IsoTpTransport<Inst, TX_CAP, RX_CAP> {
    pub fn new(config: IsoTpConfig) -> Self {
        Self {
            config,
            on_message_received: None,
            on_error: None,
            on_activity: None,
            state: IsoTpState::Idle,
            tx_buf: [0; TX_CAP],
            tx_len: 0,
            tx_sent: 0,
            tx_next_seq: 0,
            tx_block_remaining: None,
            tx_st_min_us: 0,
            rx_buf: [0; RX_CAP],
            rx_len: 0,
            rx_received: 0,
            rx_next_seq: 0,
            pending_rx: None,
            pending_control: None,
            _instant: core::marker::PhantomData,
        }
    }

    /// Resets live protocol state. Keeps `config` and the registered callbacks.
    pub fn reset(&mut self) {
        self.state = IsoTpState::Idle;
        self.tx_len = 0;
        self.tx_sent = 0;
        self.tx_next_seq = 0;
        self.tx_block_remaining = None;
        self.tx_st_min_us = 0;
        self.rx_len = 0;
        self.rx_received = 0;
        self.rx_next_seq = 0;
        self.pending_rx = None;
        self.pending_control = None;
    }

    /// Bytes sent so far / total length of the transmission in progress (`0/0` when idle).
    pub fn tx_progress(&self) -> (usize, usize) {
        (self.tx_sent, self.tx_len)
    }

    /// Bytes received so far / total length of the reception in progress (`0/0` when idle).
    pub fn rx_progress(&self) -> (usize, usize) {
        (self.rx_received, self.rx_len)
    }

    /// Microseconds the caller should wait before calling `packet_tx` again. Zero means the
    /// caller is not requesting pacing
    pub fn packet_tx_separation_us(&self) -> u32 {
        if self.state == IsoTpState::Transmitting {
            self.tx_st_min_us
        } else {
            0
        }
    }

    fn frame_cap(&self) -> usize {
        self.config.tx_frame_size.map(|dlc| dlc.len()).unwrap_or(MAX_DATA_LEN)
    }

    fn finish_frame(&self, buf: &mut [u8; MAX_DATA_LEN], len: usize) -> CanFrame {
        let mut final_len = len;
        if let Some(target) = self.config.tx_frame_size
            && final_len < target.len()
        {
            for b in &mut buf[final_len..target.len()] {
                *b = self.config.padding_byte;
            }
            final_len = target.len();
        }
        let fd = final_len > 8;
        CanFrame::new(PLACEHOLDER_ID, fd, fd, false, &buf[..final_len])
    }

    fn fail(&mut self, err: IsoTpError) -> Result<(), IsoTpError> {
        if let Some(cb) = self.on_error {
            cb(err);
        }
        Err(err)
    }

    fn queue_flow_control(&mut self, status: FlowStatus, block_size: u8, st_min: u8) {
        let mut buf = [0u8; MAX_DATA_LEN];
        let len = pci::encode_flow_control(&mut buf, status, block_size, st_min);
        self.pending_control = Some(self.finish_frame(&mut buf, len));
    }

    fn complete_rx(&mut self, len: usize) {
        let message = IsoTpMessage::new(&self.rx_buf[..len]);
        if let Some(cb) = self.on_message_received {
            cb(message.as_bytes());
        }
        self.pending_rx = Some(message);
        self.rx_len = 0;
        self.rx_received = 0;
        self.state = IsoTpState::Received;
    }

    fn finish_tx(&mut self) {
        self.state = IsoTpState::Idle;
        self.tx_len = 0;
        self.tx_sent = 0;
        self.tx_next_seq = 0;
        self.tx_block_remaining = None;
        self.tx_st_min_us = 0;
    }

    fn on_single_frame(&mut self, payload: &[u8], len: usize) -> Result<(), IsoTpError> {
        if self.pending_rx.is_some() {
            return self.fail(IsoTpError::RxNotDrained);
        }
        if len > payload.len() {
            return self.fail(IsoTpError::PciInvalid);
        }
        if len > RX_CAP {
            return self.fail(IsoTpError::MessageTooLarge);
        }

        self.rx_buf[..len].copy_from_slice(&payload[..len]);
        self.complete_rx(len);
        Ok(())
    }

    fn on_first_frame(&mut self, payload: &[u8], total_len: usize) -> Result<(), IsoTpError> {
        if self.pending_rx.is_some() {
            return self.fail(IsoTpError::RxNotDrained);
        }
        if total_len > RX_CAP {
            return self.fail(IsoTpError::MessageTooLarge);
        }

        let take = payload.len().min(total_len);
        self.rx_buf[..take].copy_from_slice(&payload[..take]);
        self.rx_received = take;
        self.rx_len = total_len;
        self.rx_next_seq = 1;
        self.state = IsoTpState::Receiving;

        let block_size = self.config.rx_desired_block_size.unwrap_or(0);
        let st_min = self.config.rx_desired_separation_us.map(pci::us_to_st_min).unwrap_or(0);
        self.queue_flow_control(FlowStatus::ContinueToSend, block_size, st_min);

        Ok(())
    }

    fn on_consecutive_frame(&mut self, payload: &[u8], seq: u8) -> Result<(), IsoTpError> {
        if self.state != IsoTpState::Receiving {
            return self.fail(IsoTpError::PciUnexpected);
        }
        if seq != self.rx_next_seq {
            return self.fail(IsoTpError::UnexpectedSequenceNumber);
        }

        let remaining = self.rx_len - self.rx_received;
        let take = payload.len().min(remaining);
        self.rx_buf[self.rx_received..self.rx_received + take].copy_from_slice(&payload[..take]);
        self.rx_received += take;
        self.rx_next_seq = pci::next_seq(seq);

        if self.rx_received >= self.rx_len {
            let len = self.rx_len;
            self.complete_rx(len);
        }

        Ok(())
    }

    fn on_flow_control(&mut self, status: FlowStatus, block_size: u8, st_min: u8) -> Result<(), IsoTpError> {
        if !matches!(self.state, IsoTpState::Transmitting | IsoTpState::AwaitingFlowControl) {
            return self.fail(IsoTpError::PciUnexpected);
        }

        match status {
            FlowStatus::ContinueToSend => {
                self.tx_block_remaining = if block_size == 0 { None } else { Some(block_size as u16) };
                self.tx_st_min_us = pci::st_min_to_us(st_min);
                self.state = IsoTpState::Transmitting;
                Ok(())
            }
            FlowStatus::Wait => {
                self.state = IsoTpState::AwaitingFlowControl;
                Ok(())
            }
            FlowStatus::Overflow => {
                self.reset();
                self.fail(IsoTpError::PartnerAborted)
            }
        }
    }

    fn next_tx_frame(&mut self) -> CanFrame {
        let frame_cap = self.frame_cap();
        let mut buf = [0u8; MAX_DATA_LEN];

        if self.tx_sent == 0 {
            if let Ok(escaped) = pci::use_escaped_single(self.config.tx_pci_format, self.tx_len) {
                let header_len = if escaped { 2 } else { 1 };
                if self.tx_len + header_len <= frame_cap {
                    let hdr = pci::encode_single_header(&mut buf, self.tx_len, escaped);
                    buf[hdr..hdr + self.tx_len].copy_from_slice(&self.tx_buf[..self.tx_len]);
                    let frame = self.finish_frame(&mut buf, hdr + self.tx_len);
                    self.finish_tx();
                    return frame;
                }
            }

            let escaped = matches!(pci::use_escaped_first(self.config.tx_pci_format, self.tx_len), Ok(true));
            let hdr = pci::encode_first_header(&mut buf, self.tx_len, escaped);
            let take = (frame_cap - hdr).min(self.tx_len);
            buf[hdr..hdr + take].copy_from_slice(&self.tx_buf[..take]);
            self.tx_sent = take;
            self.tx_next_seq = 1;
            self.state = IsoTpState::AwaitingFlowControl;
            return self.finish_frame(&mut buf, hdr + take);
        }

        let hdr = pci::encode_consecutive_header(&mut buf, self.tx_next_seq);
        let remaining = self.tx_len - self.tx_sent;
        let take = (frame_cap - hdr).min(remaining);
        buf[hdr..hdr + take].copy_from_slice(&self.tx_buf[self.tx_sent..self.tx_sent + take]);
        self.tx_sent += take;
        self.tx_next_seq = pci::next_seq(self.tx_next_seq);

        if let Some(remaining_block) = self.tx_block_remaining.as_mut() {
            *remaining_block -= 1;
            if *remaining_block == 0 {
                self.state = IsoTpState::AwaitingFlowControl;
            }
        }

        let frame = self.finish_frame(&mut buf, hdr + take);
        if self.tx_sent >= self.tx_len {
            self.finish_tx();
        }
        frame
    }
}

impl<Inst, const TX_CAP: usize, const RX_CAP: usize> Default for IsoTpTransport<Inst, TX_CAP, RX_CAP> {
    fn default() -> Self {
        Self::new(IsoTpConfig::default())
    }
}

impl<Inst: Copy, const TX_CAP: usize, const RX_CAP: usize> Transport for IsoTpTransport<Inst, TX_CAP, RX_CAP> {
    type Packet = CanFrame;
    type Message = IsoTpMessage<RX_CAP>;
    type Error = IsoTpError;
    type Instant = Inst;
    type State = IsoTpState;

    /// Ingest a CAN frame, must be filtered to only consist of messages bound for this ISO-TP channel
    fn packet_rx(&mut self, packet: Self::Packet) -> Result<(), Self::Error> {
        let data = packet.data();
        let pci = match pci::parse(data, self.config.rx_pci_format) {
            Ok(pci) => pci,
            Err(err) => return self.fail(err),
        };

        let result = match pci {
            Pci::SingleFrame { header_len, len } => self.on_single_frame(&data[header_len..], len),
            Pci::FirstFrame { header_len, total_len } => self.on_first_frame(&data[header_len..], total_len),
            Pci::ConsecutiveFrame { seq } => self.on_consecutive_frame(&data[1..], seq),
            Pci::FlowControl { status, block_size, st_min } => self.on_flow_control(status, block_size, st_min),
        };

        if result.is_ok()
            && let Some(cb) = self.on_activity
        {
            cb();
        }
        result
    }

    /// Check for available frames to send
    fn packet_tx(&mut self) -> Option<Self::Packet> {
        let frame = if let Some(frame) = self.pending_control.take() {
            Some(frame)
        } else if self.state == IsoTpState::Transmitting {
            Some(self.next_tx_frame())
        } else {
            None
        };

        if frame.is_some()
            && let Some(cb) = self.on_activity
        {
            cb();
        }
        frame
    }

    /// Write a message to the partner
    fn write(&mut self, message: Self::Message) -> Result<(), Self::Error> {
        let data = message.as_bytes();
        if data.len() > TX_CAP || pci::use_escaped_first(self.config.tx_pci_format, data.len()).is_err() {
            return self.fail(IsoTpError::MessageTooLarge);
        }

        self.tx_buf[..data.len()].copy_from_slice(data);
        self.tx_len = data.len();
        self.tx_sent = 0;
        self.tx_next_seq = 0;
        self.tx_block_remaining = None;
        self.state = IsoTpState::Transmitting;

        Ok(())
    }

    /// Read a message from the partner
    fn read(&mut self) -> Option<Self::Message> {
        let message = self.pending_rx.take();
        if message.is_some() && self.state == IsoTpState::Received {
            self.state = IsoTpState::Idle;
        }
        message
    }

    /// Check the current state
    fn poll(&mut self, _now: Self::Instant) -> Self::State {
        self.state
    }
}
