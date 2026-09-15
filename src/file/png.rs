use std::io::{self, Write};

use crate::{document::Document, graphics::Color};

const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MAX_DEFLATE_BLOCK: usize = u16::MAX as usize;

pub fn write(document: &Document, writer: &mut impl Write) -> io::Result<()> {
    let crc_table = crc_table();
    writer.write_all(SIGNATURE)?;

    let mut header = [0_u8; 13];
    header[..4].copy_from_slice(&document.width.to_be_bytes());
    header[4..8].copy_from_slice(&document.height.to_be_bytes());
    header[8] = 8;
    header[9] = 6;
    write_chunk(writer, b"IHDR", &header, &crc_table)?;

    let total_raw_bytes = u64::from(document.height) * (1 + u64::from(document.width) * 4);
    let mut written_raw_bytes = 0_u64;
    let mut first_block = true;
    let mut block = Vec::with_capacity(MAX_DEFLATE_BLOCK);
    let mut row = vec![0; 1 + document.width as usize * 4];
    let mut adler = Adler32::new();

    for y in 0..document.height {
        row[0] = 0;
        for x in 0..document.width {
            let color = document
                .composite_pixel(x, y, Color::rgba(0, 0, 0, 0))
                .expect("export coordinates stay inside the document");
            let offset = 1 + x as usize * 4;
            row[offset..offset + 4].copy_from_slice(&[
                color.red(),
                color.green(),
                color.blue(),
                color.alpha(),
            ]);
        }
        adler.update(&row);

        let mut remaining = row.as_slice();
        while !remaining.is_empty() {
            let count = remaining.len().min(MAX_DEFLATE_BLOCK - block.len());
            block.extend_from_slice(&remaining[..count]);
            remaining = &remaining[count..];
            written_raw_bytes += count as u64;
            if block.len() == MAX_DEFLATE_BLOCK {
                let final_block = written_raw_bytes == total_raw_bytes;
                write_idat_block(
                    writer,
                    &block,
                    first_block,
                    final_block,
                    adler.value(),
                    &crc_table,
                )?;
                block.clear();
                first_block = false;
            }
        }
    }
    if !block.is_empty() {
        write_idat_block(writer, &block, first_block, true, adler.value(), &crc_table)?;
    }
    write_chunk(writer, b"IEND", &[], &crc_table)
}

fn write_idat_block(
    writer: &mut impl Write,
    pixels: &[u8],
    first: bool,
    final_block: bool,
    adler: u32,
    crc_table: &[u32; 256],
) -> io::Result<()> {
    // ponytail: valid stored DEFLATE blocks; add compression only when export size matters.
    let mut data = Vec::with_capacity(
        usize::from(first) * 2 + 5 + pixels.len() + usize::from(final_block) * 4,
    );
    if first {
        data.extend_from_slice(&[0x78, 0x01]);
    }
    data.push(u8::from(final_block));
    let length = pixels.len() as u16;
    data.extend_from_slice(&length.to_le_bytes());
    data.extend_from_slice(&(!length).to_le_bytes());
    data.extend_from_slice(pixels);
    if final_block {
        data.extend_from_slice(&adler.to_be_bytes());
    }
    write_chunk(writer, b"IDAT", &data, crc_table)
}

fn write_chunk(
    writer: &mut impl Write,
    kind: &[u8; 4],
    data: &[u8],
    crc_table: &[u32; 256],
) -> io::Result<()> {
    writer.write_all(&(data.len() as u32).to_be_bytes())?;
    writer.write_all(kind)?;
    writer.write_all(data)?;
    let crc = crc32(crc32(!0, kind, crc_table), data, crc_table) ^ !0;
    writer.write_all(&crc.to_be_bytes())
}

fn crc32(mut crc: u32, bytes: &[u8], table: &[u32; 256]) -> u32 {
    for byte in bytes {
        crc = table[((crc ^ u32::from(*byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc
}

fn crc_table() -> [u32; 256] {
    let mut table = [0; 256];
    for (value, entry) in table.iter_mut().enumerate() {
        let mut crc = value as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 0 {
                crc >> 1
            } else {
                0xedb8_8320 ^ (crc >> 1)
            };
        }
        *entry = crc;
    }
    table
}

struct Adler32 {
    first: u32,
    second: u32,
}

impl Adler32 {
    fn new() -> Self {
        Self {
            first: 1,
            second: 0,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(5_552) {
            let mut first = u64::from(self.first);
            let mut second = u64::from(self.second);
            for byte in chunk {
                first += u64::from(*byte);
                second += first;
            }
            self.first = (first % 65_521) as u32;
            self.second = (second % 65_521) as u32;
        }
    }

    fn value(&self) -> u32 {
        self.second << 16 | self.first
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_rgba_png_with_stored_deflate_blocks() {
        let mut document = Document::new(2, 1, Color::rgba(0, 0, 0, 0)).unwrap();
        document
            .active_layer_mut()
            .pixels
            .stamp_circle(0.5, 0.5, 0.5, Color::rgba(10, 20, 30, 40));
        let mut png = Vec::new();
        write(&document, &mut png).unwrap();
        assert_eq!(&png[..8], SIGNATURE);

        let table = crc_table();
        let mut position = 8;
        let mut idat = Vec::new();
        while position < png.len() {
            let length =
                u32::from_be_bytes(png[position..position + 4].try_into().unwrap()) as usize;
            let kind: &[u8; 4] = png[position + 4..position + 8].try_into().unwrap();
            let data = &png[position + 8..position + 8 + length];
            let stored_crc = u32::from_be_bytes(
                png[position + 8 + length..position + 12 + length]
                    .try_into()
                    .unwrap(),
            );
            assert_eq!(
                stored_crc,
                crc32(crc32(!0, kind, &table), data, &table) ^ !0
            );
            if kind == b"IDAT" {
                idat.extend_from_slice(data);
            }
            position += 12 + length;
        }

        assert_eq!(&idat[..2], &[0x78, 0x01]);
        assert_eq!(idat[2], 1);
        let length = u16::from_le_bytes(idat[3..5].try_into().unwrap()) as usize;
        assert_eq!(length, 9);
        assert_eq!(&idat[7..7 + length], &[0, 10, 20, 30, 40, 0, 0, 0, 0]);
        let expected_adler = {
            let mut adler = Adler32::new();
            adler.update(&idat[7..7 + length]);
            adler.value()
        };
        assert_eq!(
            u32::from_be_bytes(idat[7 + length..11 + length].try_into().unwrap()),
            expected_adler
        );
    }
}
