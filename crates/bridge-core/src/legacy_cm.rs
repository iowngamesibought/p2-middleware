use std::net::Ipv4Addr;

/// Observed legacy discovery addresses, retained from the tested router profile.
pub const LEGACY_CM: [Ipv4Addr; 26] = [
    Ipv4Addr::new(69, 28, 145, 170),
    Ipv4Addr::new(69, 28, 145, 171),
    Ipv4Addr::new(208, 111, 158, 52),
    Ipv4Addr::new(208, 111, 158, 53),
    Ipv4Addr::new(208, 111, 171, 82),
    Ipv4Addr::new(208, 111, 171, 83),
    Ipv4Addr::new(68, 142, 91, 34),
    Ipv4Addr::new(68, 142, 91, 35),
    Ipv4Addr::new(208, 111, 133, 84),
    Ipv4Addr::new(208, 111, 133, 85),
    Ipv4Addr::new(68, 142, 83, 180),
    Ipv4Addr::new(68, 142, 83, 181),
    Ipv4Addr::new(68, 142, 83, 182),
    Ipv4Addr::new(68, 142, 83, 183),
    Ipv4Addr::new(72, 165, 61, 185),
    Ipv4Addr::new(72, 165, 61, 186),
    Ipv4Addr::new(72, 165, 61, 187),
    Ipv4Addr::new(72, 165, 61, 188),
    Ipv4Addr::new(79, 141, 174, 7),
    Ipv4Addr::new(79, 141, 174, 8),
    Ipv4Addr::new(79, 141, 174, 9),
    Ipv4Addr::new(79, 141, 174, 10),
    Ipv4Addr::new(81, 171, 115, 5),
    Ipv4Addr::new(81, 171, 115, 6),
    Ipv4Addr::new(81, 171, 115, 7),
    Ipv4Addr::new(81, 171, 115, 8),
];

pub fn accepts(local: std::net::SocketAddr, remote: std::net::SocketAddr, peer: Ipv4Addr) -> bool {
    remote.ip() == std::net::IpAddr::V4(peer)
        && local.port() == 27017
        && matches!(local.ip(), std::net::IpAddr::V4(ip) if LEGACY_CM.contains(&ip))
}
