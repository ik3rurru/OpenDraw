use std::io::{self, Write};

use crate::{document::Document, graphics::Color};

pub fn write(document: &Document, writer: &mut impl Write) -> io::Result<()> {
    let row_size = document.width * 3;
    let stride = (row_size + 3) & !3;
    let image_size = stride * document.height;

    writer.write_all(b"BM")?;
    writer.write_all(&(54 + image_size).to_le_bytes())?;
    writer.write_all(&[0; 4])?;
    writer.write_all(&54_u32.to_le_bytes())?;
    writer.write_all(&40_u32.to_le_bytes())?;
    writer.write_all(&(document.width as i32).to_le_bytes())?;
    writer.write_all(&(document.height as i32).to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&24_u16.to_le_bytes())?;
    writer.write_all(&0_u32.to_le_bytes())?;
    writer.write_all(&image_size.to_le_bytes())?;
    writer.write_all(&[0; 16])?;

    let mut row = vec![0; stride as usize];
    for y in (0..document.height).rev() {
        for x in 0..document.width {
            let color = document
                .composite_pixel(x, y, Color::rgba(0, 0, 0, 0))
                .expect("export coordinates stay inside the document")
                .blend_over(Color::rgb(255, 255, 255));
            let offset = (x * 3) as usize;
            row[offset..offset + 3].copy_from_slice(&[color.blue(), color.green(), color.red()]);
        }
        writer.write_all(&row)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_padded_bottom_up_bgr_pixels() {
        let mut document = Document::new(1, 1, Color::rgba(0, 0, 0, 0)).unwrap();
        document
            .active_layer_mut()
            .pixels
            .stamp_circle(0.5, 0.5, 0.5, Color::rgba(255, 0, 0, 128));
        let mut bmp = Vec::new();
        write(&document, &mut bmp).unwrap();

        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()), 58);
        assert_eq!(&bmp[54..58], &[127, 127, 255, 0]);
    }
}
