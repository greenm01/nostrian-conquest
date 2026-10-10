//! Telnet (RFC 854) command stripping for raw socket door input.
//!
//! Socket doors (`--socket-descriptor`, `--socket-port`) receive the caller's
//! telnet stream as-is, so option negotiation and subnegotiation reports
//! (NAWS, TTYPE, ...) arrive interleaved with keystrokes. The filter keeps
//! only data bytes and carries its state across reads, so commands split over
//! several reads are still removed.

const IAC: u8 = 0xFF;
const SE: u8 = 0xF0;
const SB: u8 = 0xFA;
const WILL: u8 = 0xFB;
const DONT: u8 = 0xFE;

#[derive(Debug, Default)]
pub(crate) struct TelnetFilter {
    state: TelnetState,
}

#[derive(Debug, Default, Clone, Copy)]
enum TelnetState {
    #[default]
    Data,
    /// Saw `IAC`; the next byte is a command.
    Command,
    /// Saw `IAC WILL|WONT|DO|DONT`; the next byte is the option code.
    Option,
    /// Inside `IAC SB ... IAC SE`.
    Subnegotiation,
    /// Saw `IAC` inside a subnegotiation.
    SubnegotiationCommand,
}

impl TelnetFilter {
    /// Return the data bytes of `input`, dropping telnet commands and
    /// unescaping `IAC IAC` to a literal `0xFF`.
    pub(crate) fn filter(&mut self, input: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(input.len());
        for &byte in input {
            self.state = match (self.state, byte) {
                (TelnetState::Data, IAC) => TelnetState::Command,
                (TelnetState::Data, _) => {
                    data.push(byte);
                    TelnetState::Data
                }
                (TelnetState::Command, IAC) => {
                    data.push(IAC);
                    TelnetState::Data
                }
                (TelnetState::Command, WILL..=DONT) => TelnetState::Option,
                (TelnetState::Command, SB) => TelnetState::Subnegotiation,
                // NOP, GA, AYT, and the other two-byte commands.
                (TelnetState::Command, _) => TelnetState::Data,
                (TelnetState::Option, _) => TelnetState::Data,
                (TelnetState::Subnegotiation, IAC) => TelnetState::SubnegotiationCommand,
                (TelnetState::Subnegotiation, _) => TelnetState::Subnegotiation,
                (TelnetState::SubnegotiationCommand, SE) => TelnetState::Data,
                // `IAC IAC` is an escaped 0xFF inside the subnegotiation payload.
                (TelnetState::SubnegotiationCommand, _) => TelnetState::Subnegotiation,
            };
        }
        data
    }
}
