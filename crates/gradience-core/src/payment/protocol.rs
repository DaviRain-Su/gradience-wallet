#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentProtocol {
    X402,
    Mpp,
    Hsp,
}

impl PaymentProtocol {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "x402" => Some(PaymentProtocol::X402),
            "mpp" => Some(PaymentProtocol::Mpp),
            "hsp" => Some(PaymentProtocol::Hsp),
            _ => None,
        }
    }
}
