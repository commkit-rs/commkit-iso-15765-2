/// Protocol state of an ISO-TP transport instance.
///
/// TODO: flesh out with the real ISO 15765-2 states (sending/receiving CFs, waiting on flow
/// control, error, etc.) once the state machine is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoTpState {
    Idle,
}
