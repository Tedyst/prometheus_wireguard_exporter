use crate::exporter_error::PeerEntryParseError;
use log::debug;
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Debug, Default, Clone)]
pub(crate) struct PeerEntry<'a> {
    pub public_key: &'a str,
    pub allowed_ips: &'a str,
    pub name: Option<&'a str>,
}

#[inline]
fn after_char(s: &str, c_split: char) -> &str {
    let mut p: usize = 0;
    for c in s.chars().into_iter() {
        if c == c_split {
            return &s[p + 1..];
        } else {
            p += c.len_utf8();
        }
    }
    s
}

fn from_pound_line_to_key_value(line: &str) -> Option<(&str, &str)> {
    // since the pound sign is 1 byte the below slice will work
    let line = &line[1..];
    let equals_pos = line.find('=');
    if let Some(equals_pos) = equals_pos {
        // we should trim the key
        let key = &line[..equals_pos].trim();
        // we should trim the value as well? this can be debated
        let value = &line[equals_pos + 1..].trim();
        Some((key, value))
    } else {
        None
    }
}

impl<'a> TryFrom<&[&'a str]> for PeerEntry<'a> {
    type Error = PeerEntryParseError;

    fn try_from(lines: &[&'a str]) -> Result<PeerEntry<'a>, Self::Error> {
        debug!("PeerEntry::TryFrom called with lines == {:?}", lines);

        let mut public_key = "";
        let mut allowed_ips = "";
        let mut name = None;

        for line in lines {
            let line_lowercase = line.to_lowercase();

            if line_lowercase.starts_with("publickey") {
                public_key = after_char(line, '=').trim();
            } else if line_lowercase.starts_with("allowedips") {
                allowed_ips = after_char(line, '=').trim();
            } else if line.trim().starts_with('#') {
                if let Some((key, value)) = from_pound_line_to_key_value(line) {
                    // if it's a supported key, let' map it
                    match key {
                        "friendly_name" => {
                            name = Some(value);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Sanity checks
        // If there are more than one PublicKey or AllowedIPs we won't catch it. But
        // WireGuard won't be working either so we can live with this simplification.
        if public_key == "" {
            // we return a owned String for ergonomics. This will allocate but it's ok since it's not supposed
            // to happen :)
            let lines_owned: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
            Err(PeerEntryParseError::PublicKeyNotFound { lines: lines_owned })
        } else if allowed_ips == "" {
            let lines_owned: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
            Err(PeerEntryParseError::AllowedIPsEntryNotFound { lines: lines_owned })
        } else {
            let pe = PeerEntry {
                public_key,
                allowed_ips,
                name, // name can be None
            };
            debug!("PeerEntry::TryFrom returning PeerEntryHasMap == {:?}", pe);
            Ok(pe)
        }
    }
}

pub(crate) type PeerEntryHashMap<'a> = HashMap<&'a str, PeerEntry<'a>>;

pub(crate) fn peer_entry_hashmap_try_from(
    txt: &str,
) -> Result<PeerEntryHashMap, PeerEntryParseError> {
    let mut hm = HashMap::new();

    let mut v_blocks = Vec::new();
    let mut cur_block: Option<Vec<&str>> = None;

    for line in txt.lines().into_iter() {
        if line.starts_with('[') {
            if let Some(inner_cur_block) = cur_block {
                // close the block
                v_blocks.push(inner_cur_block);
                cur_block = None;
            }

            if line == "[Peer]" {
                // start a new block
                cur_block = Some(Vec::new());
            }
        } else {
            // push the line if we are in a block (only if not empty)
            if let Some(inner_cur_block) = &mut cur_block {
                if line != "" {
                    inner_cur_block.push(line);
                }
            }
        }
    }

    if let Some(cur_block) = cur_block {
        // we have a leftover block
        v_blocks.push(cur_block);
    }

    debug!("peer_entry_hashmap_try_from v_blocks == {:?}", v_blocks);

    for block in &v_blocks {
        let p: PeerEntry = PeerEntry::try_from(&block as &[&str])?;
        hm.insert(p.public_key, p);
    }

    debug!("peer_entry_hashmap_try_from hm == {:?}", hm);

    Ok(hm)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &'static str = "
ListenPort = 51820
PrivateKey = my_super_secret_private_key
# PreUp = iptables -t nat -A POSTROUTING -s 10.70.0.0/24  -o enp7s0 -j MASQUERADE
# PostDown = iptables -t nat -D POSTROUTING -s 10.70.0.0/24  -o enp7s0 -j MASQUERADE

[Peer]
# This is a comment
# friendly_name=OnePlus 6T
# This is a comment
# This is a comment
# This is a comment
# This is a comment
PublicKey = 2S7mA0vEMethCNQrJpJKE81/JmhgtB+tHHLYQhgM6kk=
AllowedIPs = 10.70.0.2/32

[Peer]
# friendly_name=varch.local (laptop)
PublicKey = qnoxQoQI8KKMupLnSSureORV0wMmH7JryZNsmGVISzU=
AllowedIPs = 10.70.0.3/32

[Peer]
# friendly_name=cantarch
PublicKey = L2UoJZN7RmEKsMmqaJgKG0m1S2Zs2wd2ptAf+kb3008=
AllowedIPs = 10.70.0.4/32

[Peer]
# frcognoarch
PublicKey = MdVOIPKt9K2MPj/sO2NlWQbOnFJ6L/qX80mmhQwsUlA=
AllowedIPs = 10.70.0.50/32

[Peer]
# This is a comment
#               friendly_name       =               frcognowin10
# This is something
PublicKey = lqYcojJMsIZXMUw1heAFbQHBoKjCEaeo7M1WXDh/KWc=
AllowedIPs = 10.70.0.40/32

[Peer]
#friendly_name = OnePlus 5T
PublicKey = 928vO9Lf4+Mo84cWu4k1oRyzf0AR7FTGoPKHGoTMSHk=
AllowedIPs = 10.70.0.80/32
";

    const TEXT_NOPK: &'static str = "
ListenPort = 51820
PrivateKey = my_super_secret_private_key
# PreUp = iptables -t nat -A POSTROUTING -s 10.70.0.0/24  -o enp7s0 -j MASQUERADE
# PostDown = iptables -t nat -D POSTROUTING -s 10.70.0.0/24  -o enp7s0 -j MASQUERADE

[Peer]
# friendly_name = OnePlus 6T
PublicKey = 2S7mA0vEMethCNQrJpJKE81/JmhgtB+tHHLYQhgM6kk=
AllowedIPs = 10.70.0.2/32

[Peer]
# friendly_name = varch.local (laptop)
AllowedIPs = 10.70.0.3/32

[Peer]
#friendly_name= cantarch
PublicKey = L2UoJZN7RmEKsMmqaJgKG0m1S2Zs2wd2ptAf+kb3008=
AllowedIPs = 10.70.0.4/32
";

    const TEXT_AIP: &'static str = "
ListenPort = 51820
PrivateKey = my_super_secret_private_key
# PreUp = iptables -t nat -A POSTROUTING -s 10.70.0.0/24  -o enp7s0 -j MASQUERADE
# PostDown = iptables -t nat -D POSTROUTING -s 10.70.0.0/24  -o enp7s0 -j MASQUERADE

[Peer]
# friendly_name=OnePlus 6T
PublicKey = 2S7mA0vEMethCNQrJpJKE81/JmhgtB+tHHLYQhgM6kk=
AllowedIPs = 10.70.0.2/32

[Peer]
# friendly_name=varch.local (laptop)
AllowedIPs = 10.70.0.3/32
PublicKey = 6S7mA0vEMethCNQrJpJKE81/JmhgtB+tHHLYQhgM6kk=

[Peer]
# friendly_name=cantarch
PublicKey = L2UoJZN7RmEKsMmqaJgKG0m1S2Zs2wd2ptAf+kb3008=
";

    #[test]
    fn test_from_pound_line_to_key_value() {
        let a = from_pound_line_to_key_value("# ignore");
        assert_eq!(None, a);

        let a = from_pound_line_to_key_value("#           soooo much space           ");
        assert_eq!(None, a);

        let a = from_pound_line_to_key_value(
            "#           test               = This can be tricky           ",
        );
        let a = a.expect("this should have been Some!");
        assert_eq!(a.0, "test");
        assert_eq!(a.1, "This can be tricky");

        let a = from_pound_line_to_key_value("#           nasty               =");
        let a = a.expect("this should have been Some!");
        assert_eq!(a.0, "nasty");
        assert_eq!(a.1, "");

        let a = from_pound_line_to_key_value("#           nasty 2               =               ");
        let a = a.expect("this should have been Some!");
        assert_eq!(a.0, "nasty 2");
        assert_eq!(a.1, "");
    }

    #[test]
    fn test_parse_ok() {
        let a: PeerEntryHashMap = peer_entry_hashmap_try_from(TEXT).unwrap();
        println!("{:?}", a);
    }

    #[test]
    fn test_parse_friendly_name() {
        let a: PeerEntryHashMap = peer_entry_hashmap_try_from(TEXT).unwrap();
        let entry = a.get("2S7mA0vEMethCNQrJpJKE81/JmhgtB+tHHLYQhgM6kk=");
        let entry = entry.expect("this should have been Some!");
        assert_eq!(Some("OnePlus 6T"), entry.name);

        let entry = a.get("lqYcojJMsIZXMUw1heAFbQHBoKjCEaeo7M1WXDh/KWc=");
        let entry = entry.expect("this should have been Some!");
        assert_eq!(Some("frcognowin10"), entry.name);

        let entry = a.get("928vO9Lf4+Mo84cWu4k1oRyzf0AR7FTGoPKHGoTMSHk=");
        let entry = entry.expect("this should have been Some!");
        assert_eq!(Some("OnePlus 5T"), entry.name);

        let entry = a.get("MdVOIPKt9K2MPj/sO2NlWQbOnFJ6L/qX80mmhQwsUlA=");
        let entry = entry.expect("this should have been Some!");
        assert_eq!(None, entry.name);
    }

    #[test]
    #[should_panic(
        expected = "PublicKeyNotFound { lines: [\"# friendly_name = varch.local (laptop)\", \"AllowedIPs = 10.70.0.3/32\"] }"
    )]
    fn test_parse_no_public_key() {
        let _: PeerEntryHashMap = peer_entry_hashmap_try_from(TEXT_NOPK).unwrap();
    }

    #[test]
    #[should_panic(
        expected = "AllowedIPsEntryNotFound { lines: [\"# friendly_name=cantarch\", \"PublicKey = L2UoJZN7RmEKsMmqaJgKG0m1S2Zs2wd2ptAf+kb3008=\"] }"
    )]
    fn test_parse_no_allowed_ips() {
        let _: PeerEntryHashMap = peer_entry_hashmap_try_from(TEXT_AIP).unwrap();
    }
}
