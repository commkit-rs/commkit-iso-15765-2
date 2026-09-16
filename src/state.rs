#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoTpState {
    Idle,
    Transmitting,
    AwaitingFlowControl,
    Receiving,
    Received,
}
