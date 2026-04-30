//! Network interface discovery for agent registration.
//!
//! Enumerates all non-loopback, non-link-local IP addresses on the host and
//! identifies the recommended address using the OS routing table (UDP socket
//! trick — no packets are actually sent).

use crate::openapi;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use tracing::{debug, warn};

/// Determine the source IP the OS would use when connecting to `server_addr`.
///
/// Opens a UDP socket and calls `connect()` + `local_addr()`. The OS consults
/// its routing table to resolve the source address without sending any traffic.
/// Returns `None` if the resolution fails.
fn routing_table_source_ip(server_addr: &str) -> Option<IpAddr> {
    // Parse the server address into a SocketAddr. Strip any scheme/path first.
    let host_port = extract_host_port(server_addr);

    // Try IPv4 first, fall back to IPv6.
    let socket_addr: SocketAddr = host_port.parse().ok()?;

    let bind_addr: SocketAddr = match socket_addr {
        SocketAddr::V4(_) => "0.0.0.0:0".parse().ok()?,
        SocketAddr::V6(_) => "[::]:0".parse().ok()?,
    };

    let socket = UdpSocket::bind(bind_addr).ok()?;
    socket.connect(socket_addr).ok()?;
    let local = socket.local_addr().ok()?;
    Some(local.ip())
}

/// Extract `host:port` from a URL string like `https://192.168.1.1:8080/v1`.
/// Falls back to the original string if parsing fails.
fn extract_host_port(url: &str) -> String {
    // Strip scheme
    let without_scheme = url
        .find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url);

    // Strip path/query/fragment
    let host_port = without_scheme
        .find('/')
        .map(|i| &without_scheme[..i])
        .unwrap_or(without_scheme);

    host_port.to_string()
}

/// Returns `true` if the address should be excluded from the registration list.
///
/// Excluded ranges:
/// - IPv4 loopback: `127.0.0.0/8`
/// - IPv6 loopback: `::1`
/// - IPv4 link-local: `169.254.0.0/16`
/// - IPv6 link-local: `fe80::/10`
fn is_excluded(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_loopback() || is_ipv4_link_local(v4),
        IpAddr::V6(v6) => v6.is_loopback() || is_ipv6_link_local(v6),
    }
}

/// Returns `true` for `169.254.0.0/16`.
fn is_ipv4_link_local(addr: &Ipv4Addr) -> bool {
    addr.octets()[0] == 169 && addr.octets()[1] == 254
}

/// Returns `true` for `fe80::/10`.
fn is_ipv6_link_local(addr: &std::net::Ipv6Addr) -> bool {
    let segments = addr.segments();
    // fe80::/10 — first 10 bits are 1111111010
    (segments[0] & 0xffc0) == 0xfe80
}

/// Collect all non-loopback, non-link-local network interfaces on this host.
///
/// `server_url` is used to determine the recommended address via the OS routing
/// table. The address the OS would select as source IP for a connection toward
/// the server is marked `recommended = true`.
///
/// Returns an empty `Vec` (with a warning log) if interface enumeration fails.
pub fn collect_interfaces(server_url: &str) -> Vec<openapi::AgentNetworkInterface> {
    let recommended_ip = routing_table_source_ip(server_url);
    debug!(?recommended_ip, "OS-selected source IP toward server");

    let ifaces = match if_addrs::get_if_addrs() {
        Ok(i) => i,
        Err(e) => {
            warn!("Failed to enumerate network interfaces: {}", e);
            return Vec::new();
        }
    };

    ifaces
        .into_iter()
        .filter_map(|iface| {
            let ip = iface.ip();
            if is_excluded(&ip) {
                return None;
            }

            let family = match ip {
                IpAddr::V4(_) => openapi::IpAddressFamily::Ipv4,
                IpAddr::V6(_) => openapi::IpAddressFamily::Ipv6,
            };

            let recommended = recommended_ip.map_or(false, |rec| rec == ip);

            Some(openapi::AgentNetworkInterface {
                ip: ip.to_string(),
                iface: iface.name.clone(),
                family,
                recommended,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    // ── extract_host_port ────────────────────────────────────────────────────

    mod extract_host_port_tests {
        use super::*;

        #[test]
        fn test_https_url_with_path() {
            assert_eq!(
                extract_host_port("https://192.168.1.1:8080/v1"),
                "192.168.1.1:8080"
            );
        }

        #[test]
        fn test_http_url_no_path() {
            assert_eq!(
                extract_host_port("http://10.0.0.1:80"),
                "10.0.0.1:80"
            );
        }

        #[test]
        fn test_bare_host_port() {
            assert_eq!(extract_host_port("10.0.0.1:443"), "10.0.0.1:443");
        }

        #[test]
        fn test_url_with_query() {
            assert_eq!(
                extract_host_port("https://server.example.com:9000/api/v1?foo=bar"),
                "server.example.com:9000"
            );
        }
    }

    // ── is_ipv4_link_local ───────────────────────────────────────────────────

    mod is_ipv4_link_local_tests {
        use super::*;

        #[test]
        fn test_link_local_start() {
            assert!(is_ipv4_link_local(&Ipv4Addr::new(169, 254, 0, 1)));
        }

        #[test]
        fn test_link_local_end() {
            assert!(is_ipv4_link_local(&Ipv4Addr::new(169, 254, 255, 254)));
        }

        #[test]
        fn test_not_link_local() {
            assert!(!is_ipv4_link_local(&Ipv4Addr::new(192, 168, 1, 1)));
            assert!(!is_ipv4_link_local(&Ipv4Addr::new(10, 0, 0, 1)));
            assert!(!is_ipv4_link_local(&Ipv4Addr::new(169, 255, 0, 1)));
        }
    }

    // ── is_ipv6_link_local ───────────────────────────────────────────────────

    mod is_ipv6_link_local_tests {
        use super::*;

        #[test]
        fn test_fe80_is_link_local() {
            let addr: Ipv6Addr = "fe80::1".parse().unwrap();
            assert!(is_ipv6_link_local(&addr));
        }

        #[test]
        fn test_fe80_full_range() {
            // fe80::/10 — fe80 through febf
            let addr: Ipv6Addr = "febf::1".parse().unwrap();
            assert!(is_ipv6_link_local(&addr));
        }

        #[test]
        fn test_non_link_local_ipv6() {
            let addr: Ipv6Addr = "2001:db8::1".parse().unwrap();
            assert!(!is_ipv6_link_local(&addr));
            let addr2: Ipv6Addr = "fc00::1".parse().unwrap();
            assert!(!is_ipv6_link_local(&addr2));
        }
    }

    // ── is_excluded ──────────────────────────────────────────────────────────

    mod is_excluded_tests {
        use super::*;

        #[test]
        fn test_ipv4_loopback_excluded() {
            assert!(is_excluded(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        }

        #[test]
        fn test_ipv6_loopback_excluded() {
            assert!(is_excluded(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
        }

        #[test]
        fn test_ipv4_link_local_excluded() {
            assert!(is_excluded(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 5))));
        }

        #[test]
        fn test_ipv6_link_local_excluded() {
            let addr: IpAddr = "fe80::1".parse().unwrap();
            assert!(is_excluded(&addr));
        }

        #[test]
        fn test_regular_ipv4_not_excluded() {
            assert!(!is_excluded(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))));
            assert!(!is_excluded(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        }

        #[test]
        fn test_regular_ipv6_not_excluded() {
            let addr: IpAddr = "2001:db8::1".parse().unwrap();
            assert!(!is_excluded(&addr));
        }
    }

    // ── collect_interfaces ───────────────────────────────────────────────────

    mod collect_interfaces_tests {
        use super::*;

        #[test]
        fn test_no_loopback_in_results() {
            // Use a real server URL — the routing table probe may fail but
            // collect_interfaces must still return a non-empty list on any
            // machine with at least one non-loopback interface.
            let ifaces = collect_interfaces("http://8.8.8.8:80");
            for iface in &ifaces {
                assert_ne!(iface.ip, "127.0.0.1", "loopback must be excluded");
                assert_ne!(iface.ip, "::1", "IPv6 loopback must be excluded");
                assert!(
                    !iface.ip.starts_with("169.254."),
                    "link-local must be excluded"
                );
            }
        }

        #[test]
        fn test_at_most_one_recommended() {
            let ifaces = collect_interfaces("http://8.8.8.8:80");
            let recommended_count = ifaces.iter().filter(|i| i.recommended).count();
            assert!(
                recommended_count <= 1,
                "at most one interface should be recommended, got {}",
                recommended_count
            );
        }

        #[test]
        fn test_family_matches_ip() {
            let ifaces = collect_interfaces("http://8.8.8.8:80");
            for iface in &ifaces {
                let ip: IpAddr = iface.ip.parse().expect("ip field should be a valid IP");
                match iface.family {
                    openapi::IpAddressFamily::Ipv4 => {
                        assert!(ip.is_ipv4(), "family=ipv4 but IP is not IPv4: {}", iface.ip)
                    }
                    openapi::IpAddressFamily::Ipv6 => {
                        assert!(ip.is_ipv6(), "family=ipv6 but IP is not IPv6: {}", iface.ip)
                    }
                }
            }
        }

        #[test]
        fn test_invalid_server_url_does_not_panic() {
            // When the server URL can't be parsed, recommended falls back to false.
            let ifaces = collect_interfaces("not-a-valid-url");
            // Should still return interface list, just no recommended entry.
            let recommended_count = ifaces.iter().filter(|i| i.recommended).count();
            assert_eq!(recommended_count, 0);
        }
    }
}
