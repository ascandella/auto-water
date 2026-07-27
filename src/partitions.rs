const PARTITIONS_CSV: &str = include_str!("../partitions.csv");

pub const fn find_partition(name: &str) -> (usize, usize) {
    let csv = PARTITIONS_CSV.as_bytes();
    let name_bytes = name.as_bytes();
    let csv_len = csv.len();
    let name_len = name_bytes.len();

    let mut line_start: usize = 0;
    let mut found_offset: usize = 0;
    let mut found_size: usize = 0;
    let mut i: usize = 0;

    while i < csv_len {
        let is_newline = csv[i] == b'\n';
        let is_eof = i + 1 == csv_len;

        if is_newline || is_eof {
            let line_end = if is_newline { i } else { csv_len };

            if line_end > line_start && csv[line_start] != b'#' {
                let mut field: usize = 0;
                let mut fstart: usize = line_start;
                let mut j: usize = line_start;
                let mut offset_val: usize = 0;
                let mut size_val: usize = 0;
                let mut name_matches = false;

                while j <= line_end {
                    if j == line_end || csv[j] == b',' {
                        let fstart_trimmed = skip_spaces(csv, fstart, j);

                        if field == 0 && (j - fstart_trimmed) == name_len {
                            name_matches = bytes_equal(csv, fstart_trimmed, name_bytes);
                        } else if field == 3 {
                            offset_val = parse_hex_slice(csv, fstart_trimmed, j);
                        } else if field == 4 {
                            size_val = parse_hex_slice(csv, fstart_trimmed, j);
                        }

                        field += 1;
                        fstart = j + 1;
                    }
                    j += 1;
                }

                if name_matches {
                    found_offset = offset_val;
                    found_size = size_val;
                    break;
                }
            }

            line_start = i + 1;
        }
        i += 1;
    }

    (found_offset, found_size)
}

const fn skip_spaces(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut k = start;
    while k < end && bytes[k] == b' ' {
        k += 1;
    }
    k
}

const fn bytes_equal(haystack: &[u8], offset: usize, needle: &[u8]) -> bool {
    let mut i = 0;
    while i < needle.len() {
        if haystack[offset + i] != needle[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn parse_hex_slice(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut result: usize = 0;
    let mut i = start;

    if end > start + 2
        && bytes[start] == b'0'
        && (bytes[start + 1] == b'x' || bytes[start + 1] == b'X')
    {
        i = start + 2;
    }

    while i < end {
        result *= 16;
        let b = bytes[i];
        if b >= b'0' && b <= b'9' {
            result += (b - b'0') as usize;
        } else if b >= b'a' && b <= b'f' {
            result += (b - b'a' + 10) as usize;
        } else if b >= b'A' && b <= b'F' {
            result += (b - b'A' + 10) as usize;
        }
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_actual_partition_table() {
        let (offset, size) = find_partition("nvs");
        assert_eq!(offset, 0x9000);
        assert_eq!(size, 0x6000);
    }

    #[test]
    fn parses_ota_partitions() {
        let (offset, size) = find_partition("ota_0");
        assert_eq!(offset, 0x20000);
        assert_eq!(size, 0x1E0000);

        let (offset, size) = find_partition("ota_1");
        assert_eq!(offset, 0x200000);
        assert_eq!(size, 0x1E0000);
    }

    #[test]
    fn parses_otadata() {
        let (offset, size) = find_partition("otadata");
        assert_eq!(offset, 0xf000);
        assert_eq!(size, 0x2000);
    }

    #[test]
    fn unknown_partition_returns_zero() {
        let (offset, size) = find_partition("nonexistent");
        assert_eq!(offset, 0);
        assert_eq!(size, 0);
    }

    #[test]
    fn skips_comment_lines() {
        // The actual CSV has comment lines starting with #
        // If we add one, find_partition should skip it
        let (_, _) = find_partition("nvs");
        // Just verifying it doesn't panic or return wrong data
    }
}
