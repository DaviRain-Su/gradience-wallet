#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentProtocol {
    Mpp,
    Hsp,
}

impl PaymentProtocol {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mpp" => Some(PaymentProtocol::Mpp),
            "hsp" => Some(PaymentProtocol::Hsp),
            _ => None,
        }
    }
}

impl std::str::FromStr for PaymentProtocol {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
}
