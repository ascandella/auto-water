#![no_std]

pub fn append_bytes(buf: &mut [u8], pos: usize, data: &[u8]) -> usize {
    buf[pos..pos + data.len()].copy_from_slice(data);
    pos + data.len()
}

pub fn parse_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query
        .split('&')
        .find(|p| p.starts_with(key) && p.as_bytes().get(key.len()) == Some(&b'='))
        .and_then(|p| p.get(key.len() + 1..))
}

#[allow(clippy::result_unit_err)]
pub fn parse_u64(s: &str) -> Result<u64, ()> {
    let mut n: u64 = 0;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return Err(());
        }
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as u64))
            .ok_or(())?;
    }
    Ok(n)
}

pub fn format_u64(mut n: u64, buf: &mut [u8; 20]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    let len = buf.len() - i;
    buf.copy_within(i.., 0);
    len
}

pub fn format_u32(mut n: u32, buf: &mut [u8; 10]) -> &[u8] {
    if n == 0 {
        return b"0";
    }
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_param_basic() {
        assert_eq!(
            parse_param("interval=3600&duration=10", "interval"),
            Some("3600")
        );
        assert_eq!(
            parse_param("interval=3600&duration=10", "duration"),
            Some("10")
        );
    }

    #[test]
    fn parse_param_missing() {
        assert_eq!(parse_param("interval=3600", "duration"), None);
    }

    #[test]
    fn parse_param_empty() {
        assert_eq!(parse_param("", "interval"), None);
    }

    #[test]
    fn parse_param_prefix_no_match() {
        assert_eq!(parse_param("intervals=3600", "interval"), None);
    }

    #[test]
    fn parse_param_single() {
        assert_eq!(parse_param("key=val", "key"), Some("val"));
    }

    #[test]
    fn parse_param_empty_value() {
        assert_eq!(parse_param("key=", "key"), Some(""));
    }

    #[test]
    fn parse_u64_zero() {
        assert_eq!(parse_u64("0"), Ok(0));
    }

    #[test]
    fn parse_u64_normal() {
        assert_eq!(parse_u64("3600"), Ok(3600));
        assert_eq!(parse_u64("172800"), Ok(172800));
    }

    #[test]
    fn parse_u64_max() {
        assert_eq!(parse_u64("18446744073709551615"), Ok(u64::MAX));
    }

    #[test]
    fn parse_u64_overflow() {
        assert_eq!(parse_u64("18446744073709551616"), Err(()));
    }

    #[test]
    fn parse_u64_non_digit() {
        assert_eq!(parse_u64("abc"), Err(()));
        assert_eq!(parse_u64("12a34"), Err(()));
    }

    #[test]
    fn parse_u64_empty() {
        assert_eq!(parse_u64(""), Ok(0));
    }

    #[test]
    fn format_u64_zero() {
        let mut buf = [0u8; 20];
        let n = format_u64(0, &mut buf);
        assert_eq!(&buf[..n], b"0");
    }

    #[test]
    fn format_u64_normal() {
        let mut buf = [0u8; 20];
        let n = format_u64(172800, &mut buf);
        assert_eq!(&buf[..n], b"172800");
    }

    #[test]
    fn format_u64_single_digit() {
        let mut buf = [0u8; 20];
        let n = format_u64(7, &mut buf);
        assert_eq!(&buf[..n], b"7");
    }

    #[test]
    fn format_u64_max() {
        let mut buf = [0u8; 20];
        let n = format_u64(u64::MAX, &mut buf);
        assert_eq!(&buf[..n], b"18446744073709551615");
    }

    #[test]
    fn format_u64_roundtrip() {
        let mut buf = [0u8; 20];
        for val in [0, 1, 9, 10, 99, 100, 12345, 172800, u64::MAX] {
            let n = format_u64(val, &mut buf);
            let s = core::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(s.parse::<u64>().unwrap(), val);
        }
    }

    #[test]
    fn format_u32_zero() {
        let mut buf = [0u8; 10];
        assert_eq!(format_u32(0, &mut buf), b"0");
    }

    #[test]
    fn format_u32_normal() {
        let mut buf = [0u8; 10];
        assert_eq!(format_u32(2048, &mut buf), b"2048");
    }

    #[test]
    fn format_u32_max() {
        let mut buf = [0u8; 10];
        assert_eq!(format_u32(u32::MAX, &mut buf), b"4294967295");
    }

    #[test]
    fn append_bytes_basic() {
        let mut buf = [0u8; 20];
        let pos = append_bytes(&mut buf, 0, b"hello");
        assert_eq!(pos, 5);
        assert_eq!(&buf[..5], b"hello");
    }

    #[test]
    fn append_bytes_chained() {
        let mut buf = [0u8; 20];
        let pos = append_bytes(&mut buf, 0, b"aaa");
        let pos = append_bytes(&mut buf, pos, b"bbb");
        assert_eq!(pos, 6);
        assert_eq!(&buf[..6], b"aaabbb");
    }
}
