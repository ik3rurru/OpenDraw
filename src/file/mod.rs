//! ODRW v1 stores `ODRW`, version, dimensions, layer count and active layer,
//! followed by each UTF-8 name, visibility, opacity and `0xAARRGGBB` pixels.
//! Every integer is little-endian; layer pixels are stored from top-left by rows.

mod bmp;
mod png;

use std::{
    ffi::OsString,
    fmt,
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use crate::document::{Document, Layer, MAX_LAYERS, MAX_PIXELS, PixelBuffer};

const MAGIC: &[u8; 4] = b"ODRW";
const VERSION: u32 = 1;
const MAX_LAYER_NAME_BYTES: usize = 4_096;
const PIXELS_PER_CHUNK: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Bmp,
    Png,
}

impl ImageFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Bmp => "bmp",
            Self::Png => "png",
        }
    }
}

#[derive(Debug)]
pub enum OdrawError {
    Io(io::Error),
    InvalidFile,
    UnsupportedVersion(u32),
    UnexpectedEndOfFile,
    InvalidDimensions,
    AllocationFailed,
}

impl fmt::Display for OdrawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "file error: {error}"),
            Self::InvalidFile => formatter.write_str("invalid OpenDraw file"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported OpenDraw version {version}")
            }
            Self::UnexpectedEndOfFile => formatter.write_str("unexpected end of OpenDraw file"),
            Self::InvalidDimensions => formatter.write_str("invalid document dimensions"),
            Self::AllocationFailed => formatter.write_str("could not allocate document pixels"),
        }
    }
}

impl std::error::Error for OdrawError {}

impl From<io::Error> for OdrawError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn save(document: &Document, path: &Path) -> Result<(), OdrawError> {
    validate_document(document)?;
    write_atomically(path, |writer| write_document(document, writer))
}

pub fn load(path: &Path) -> Result<Document, OdrawError> {
    read_document(&mut BufReader::new(File::open(path)?))
}

pub fn export(document: &Document, path: &Path, format: ImageFormat) -> io::Result<()> {
    write_atomically(path, |writer| match format {
        ImageFormat::Bmp => bmp::write(document, writer),
        ImageFormat::Png => png::write(document, writer),
    })
}

fn write_atomically<E>(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> Result<(), E>,
) -> Result<(), E>
where
    E: From<io::Error>,
{
    let (temporary_path, file) = temporary_file(path).map_err(E::from)?;
    let result = (|| {
        let mut writer = BufWriter::new(file);
        write(&mut writer)?;
        writer.flush().map_err(E::from)?;
        writer.get_ref().sync_all().map_err(E::from)?;
        drop(writer);
        crate::platform::replace_file(&temporary_path, path).map_err(E::from)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary_path);
    }
    result
}

fn temporary_file(path: &Path) -> io::Result<(PathBuf, File)> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    for attempt in 0..100 {
        let mut temporary_name = OsString::from(name);
        temporary_name.push(format!(".opendraw-{}-{attempt}.tmp", std::process::id()));
        let temporary_path = path.with_file_name(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a temporary file",
    ))
}

fn write_document(document: &Document, writer: &mut impl Write) -> Result<(), OdrawError> {
    writer.write_all(MAGIC)?;
    write_u32(writer, VERSION)?;
    write_u32(writer, document.width)?;
    write_u32(writer, document.height)?;
    write_u32(
        writer,
        u32::try_from(document.layers.len()).map_err(|_| OdrawError::InvalidDimensions)?,
    )?;
    write_u32(
        writer,
        u32::try_from(document.active_layer).map_err(|_| OdrawError::InvalidDimensions)?,
    )?;

    let mut bytes = [0_u8; PIXELS_PER_CHUNK * 4];
    for layer in &document.layers {
        let name = layer.name.as_bytes();
        write_u32(
            writer,
            u32::try_from(name.len()).map_err(|_| OdrawError::InvalidFile)?,
        )?;
        writer.write_all(name)?;
        writer.write_all(&[u8::from(layer.visible), layer.opacity])?;
        for pixels in layer.pixels.pixels.chunks(PIXELS_PER_CHUNK) {
            for (output, pixel) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(pixels) {
                output.copy_from_slice(&pixel.to_le_bytes());
            }
            writer.write_all(&bytes[..pixels.len() * 4])?;
        }
    }
    Ok(())
}

fn read_document(reader: &mut impl Read) -> Result<Document, OdrawError> {
    if read_array::<4>(reader)? != *MAGIC {
        return Err(OdrawError::InvalidFile);
    }
    let version = read_u32(reader)?;
    if version != VERSION {
        return Err(OdrawError::UnsupportedVersion(version));
    }

    let width = read_u32(reader)?;
    let height = read_u32(reader)?;
    let layer_count = read_u32(reader)? as usize;
    let active_layer = read_u32(reader)? as usize;
    let pixels_per_layer = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || layer_count == 0
        || layer_count > MAX_LAYERS
        || active_layer >= layer_count
        || pixels_per_layer
            .checked_mul(layer_count as u64)
            .is_none_or(|total| total > MAX_PIXELS)
    {
        return Err(OdrawError::InvalidDimensions);
    }
    let pixels_per_layer =
        usize::try_from(pixels_per_layer).map_err(|_| OdrawError::InvalidDimensions)?;

    let mut layers = Vec::new();
    layers
        .try_reserve_exact(layer_count)
        .map_err(|_| OdrawError::AllocationFailed)?;
    let mut bytes = [0_u8; PIXELS_PER_CHUNK * 4];
    for _ in 0..layer_count {
        let name_length = read_u32(reader)? as usize;
        if name_length > MAX_LAYER_NAME_BYTES {
            return Err(OdrawError::InvalidFile);
        }
        let mut name = vec![0; name_length];
        read_exact(reader, &mut name)?;
        let name = String::from_utf8(name).map_err(|_| OdrawError::InvalidFile)?;
        let [visible, opacity] = read_array::<2>(reader)?;
        let visible = match visible {
            0 => false,
            1 => true,
            _ => return Err(OdrawError::InvalidFile),
        };

        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(pixels_per_layer)
            .map_err(|_| OdrawError::AllocationFailed)?;
        let mut remaining = pixels_per_layer;
        while remaining > 0 {
            let count = remaining.min(PIXELS_PER_CHUNK);
            read_exact(reader, &mut bytes[..count * 4])?;
            pixels.extend(
                bytes[..count * 4]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|pixel| u32::from_le_bytes(*pixel)),
            );
            remaining -= count;
        }
        layers.push(Layer {
            name,
            visible,
            opacity,
            pixels: PixelBuffer {
                width,
                height,
                pixels,
            },
        });
    }

    let mut trailing = [0];
    match reader.read(&mut trailing) {
        Ok(0) => {}
        Ok(_) => return Err(OdrawError::InvalidFile),
        Err(error) => return Err(OdrawError::Io(error)),
    }
    Document::from_layers(width, height, layers, active_layer)
        .map_err(|_| OdrawError::InvalidDimensions)
}

fn validate_document(document: &Document) -> Result<(), OdrawError> {
    let pixels_per_layer = u64::from(document.width) * u64::from(document.height);
    let expected = usize::try_from(pixels_per_layer).map_err(|_| OdrawError::InvalidDimensions)?;
    if document.width == 0
        || document.height == 0
        || document.layers.is_empty()
        || document.layers.len() > MAX_LAYERS
        || document.active_layer >= document.layers.len()
        || pixels_per_layer
            .checked_mul(document.layers.len() as u64)
            .is_none_or(|total| total > MAX_PIXELS)
        || document.layers.iter().any(|layer| {
            layer.name.len() > MAX_LAYER_NAME_BYTES
                || layer.pixels.width != document.width
                || layer.pixels.height != document.height
                || layer.pixels.pixels.len() != expected
        })
    {
        return Err(OdrawError::InvalidDimensions);
    }
    Ok(())
}

fn write_u32(writer: &mut impl Write, value: u32) -> Result<(), OdrawError> {
    writer.write_all(&value.to_le_bytes()).map_err(Into::into)
}

fn read_u32(reader: &mut impl Read) -> Result<u32, OdrawError> {
    Ok(u32::from_le_bytes(read_array(reader)?))
}

fn read_array<const SIZE: usize>(reader: &mut impl Read) -> Result<[u8; SIZE], OdrawError> {
    let mut bytes = [0; SIZE];
    read_exact(reader, &mut bytes)?;
    Ok(bytes)
}

fn read_exact(reader: &mut impl Read, bytes: &mut [u8]) -> Result<(), OdrawError> {
    reader.read_exact(bytes).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            OdrawError::UnexpectedEndOfFile
        } else {
            OdrawError::Io(error)
        }
    })
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use crate::graphics::Color;

    use super::*;

    #[test]
    fn round_trips_documents_and_rejects_corrupt_headers() {
        let mut document = Document::new(3, 2, Color::rgb(250, 250, 250)).unwrap();
        document.active_layer_mut().name = String::from("FONDO");
        document.add_layer().unwrap();
        document.active_layer_mut().name = String::from("TINTA");
        document.active_layer_mut().opacity = 137;
        document.active_layer_mut().visible = false;
        document
            .active_layer_mut()
            .pixels
            .stamp_circle(1, 1, 0, Color::rgba(10, 20, 30, 40));

        let mut bytes = Vec::new();
        write_document(&document, &mut bytes).unwrap();
        let loaded = read_document(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!((loaded.width, loaded.height), (3, 2));
        assert_eq!(loaded.active_layer, 1);
        assert_eq!(loaded.layers[0].name, "FONDO");
        assert_eq!(loaded.layers[1].name, "TINTA");
        assert!(!loaded.layers[1].visible);
        assert_eq!(loaded.layers[1].opacity, 137);
        assert_eq!(
            loaded.layers[1].pixels.get_pixel(1, 1),
            Some(Color::rgba(10, 20, 30, 40))
        );

        let mut bad_magic = bytes.clone();
        bad_magic[0] = b'X';
        assert!(matches!(
            read_document(&mut Cursor::new(bad_magic)),
            Err(OdrawError::InvalidFile)
        ));
        let mut future = bytes.clone();
        future[4..8].copy_from_slice(&2_u32.to_le_bytes());
        assert!(matches!(
            read_document(&mut Cursor::new(future)),
            Err(OdrawError::UnsupportedVersion(2))
        ));
        assert!(matches!(
            read_document(&mut Cursor::new(&bytes[..10])),
            Err(OdrawError::UnexpectedEndOfFile)
        ));
    }

    #[test]
    fn atomic_write_preserves_the_previous_file_on_failure() {
        let path = std::env::temp_dir().join(format!(
            "opendraw-{}-atomic-write.odraw",
            std::process::id()
        ));
        std::fs::write(&path, b"original").unwrap();

        write_atomically(&path, |writer| writer.write_all(b"replacement")).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");

        let result: io::Result<()> = write_atomically(&path, |writer| {
            writer.write_all(b"partial")?;
            Err(io::Error::other("simulated write failure"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        std::fs::remove_file(path).unwrap();
    }
}
