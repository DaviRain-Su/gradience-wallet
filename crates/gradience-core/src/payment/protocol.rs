#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentProtocol {
    Mpp,
    Hsp,
}

impl PaymentProtocol {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mpp" => Some(PaymentProtocol::Mpp),
            "hsp" => Some(PaymentProtocol::Hsp),
            _ => None,
        }
    }
}
