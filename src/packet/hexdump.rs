use std::fmt;

pub const DEFAULT_BYTES_PER_LINE: usize = 16;

pub fn hexdump(data: &[u8]) -> String {
    hexdump_with_offset(data, 0, DEFAULT_BYTES_PER_LINE)
}

pub fn hexdump_with_offset(data: &[u8], start_offset: usize, bytes_per_line: usize) -> String {
    if data.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let _total_len = data.len();

    for (i, chunk) in data.chunks(bytes_per_line).enumerate() {
        let offset = start_offset + (i * bytes_per_line);
        output.push_str(&format!("{:08x}  ", offset));

        let mut hex_part = String::new();
        let mut ascii_part = String::new();

        for (j, &byte) in chunk.iter().enumerate() {
            if j > 0 && j % 8 == 0 {
                hex_part.push(' ');
            }
            hex_part.push_str(&format!("{:02x} ", byte));

            if byte.is_ascii_graphic() || byte == b' ' {
                ascii_part.push(byte as char);
            } else {
                ascii_part.push('.');
            }
        }

        let padding_len = bytes_per_line - chunk.len();
        for j in 0..padding_len {
            if (chunk.len() + j) % 8 == 0 && j > 0 {
                hex_part.push(' ');
            }
            hex_part.push_str("   ");
        }

        output.push_str(&hex_part);
        output.push_str(" |");
        output.push_str(&ascii_part);
        output.push_str("|\n");
    }

    output
}

pub struct HexDumper<W> {
    writer: W,
    bytes_per_line: usize,
    offset: usize,
    current_line_offset: usize,
}

impl<W: fmt::Write> HexDumper<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            bytes_per_line: DEFAULT_BYTES_PER_LINE,
            offset: 0,
            current_line_offset: 0,
        }
    }

    pub fn with_bytes_per_line(mut self, bytes_per_line: usize) -> Self {
        self.bytes_per_line = bytes_per_line;
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self.current_line_offset = offset;
        self
    }

    pub fn write_packet(&mut self, data: &[u8]) -> fmt::Result {
        for (i, chunk) in data.chunks(self.bytes_per_line).enumerate() {
            let line_offset = self.current_line_offset + (i * self.bytes_per_line);
            write!(self.writer, "{:08x}  ", line_offset)?;

            let mut hex_part = String::new();
            let mut ascii_part = String::new();

            for (j, &byte) in chunk.iter().enumerate() {
                if j > 0 && j % 8 == 0 {
                    hex_part.push(' ');
                }
                hex_part.push_str(&format!("{:02x} ", byte));

                if byte.is_ascii_graphic() || byte == b' ' {
                    ascii_part.push(byte as char);
                } else {
                    ascii_part.push('.');
                }
            }

            let padding_len = self.bytes_per_line - chunk.len();
            for j in 0..padding_len {
                if (chunk.len() + j) % 8 == 0 && j > 0 {
                    hex_part.push(' ');
                }
                hex_part.push_str("   ");
            }

            write!(self.writer, "{} |{}|\n", hex_part, ascii_part)?;
        }

        self.current_line_offset += data.len();
        Ok(())
    }

    pub fn finish(mut self) -> fmt::Result {
        write!(self.writer, "{:08x}\n", self.current_line_offset)?;
        Ok(())
    }
}

impl HexDumper<String> {
    pub fn to_string(data: &[u8]) -> String {
        let mut dumper = Self::new(String::new());
        let _ = dumper.write_packet(data);
        dumper.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexdump_empty() {
        assert_eq!(hexdump(b""), "");
    }

    #[test]
    fn test_hexdump_basic() {
        let data = b"Hello, World!";
        let output = hexdump(data);
        assert!(output.contains("48656c6c 6f2c2057 6f726c64 21"));
        assert!(output.contains("Hello, World!"));
    }

    #[test]
    fn test_hexdump_with_offset() {
        let data = b"Test";
        let output = hexdump_with_offset(data, 0x1000, 16);
        assert!(output.starts_with("00001000"));
    }

    #[test]
    fn test_hexdump_non_printable() {
        let data = [0x00, 0x01, 0x02, 0x7f, 0xff];
        let output = hexdump(&data);
        assert!(output.contains("....."));
    }

    #[test]
    fn test_hexdump_16_bytes() {
        let data: Vec<u8> = (0..16).collect();
        let output = hexdump(&data);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(output.contains("00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f"));
    }

    #[test]
    fn test_hexdump_over_16_bytes() {
        let data: Vec<u8> = (0..32).collect();
        let output = hexdump(&data);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
    }
}
