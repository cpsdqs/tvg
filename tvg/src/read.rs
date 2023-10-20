use crate::layer::{LayerData, read_layer_data};
use crate::palette::{PaletteData, read_palette_data};
use crate::util::read_encoded_data;
use byteorder::{LE, ReadBytesExt};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::io::{self, BufRead, Read};
use thiserror::Error;

pub const MAGIC: [u8; 8] = *b"OTVGfull";
pub const TVG_VERSION: u32 = 1009;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected magic: {0:?}")]
    UnexpectedMagic([u8; 8]),
    #[error("unexpected version: {0}")]
    UnexpectedVersion(u32),
    #[error("mystery: {0}")]
    UnknownMystery(String),
    #[error("unknown file tag: {0:08x?}")]
    UnknownFileTag(u32),
    #[error("unknown layer tag: {0:08x?}")]
    UnknownLayerTag(u32),
    #[error("unknown shape type: {0:04x?}")]
    UnknownShapeType(u16),
    #[error("unknown shape component type: {0:02x?}")]
    UnknownComponentType(u8),
    #[error("unknown shape component tag: {0:08x?}")]
    UnknownComponentTag(u32),
    #[error("unknown palette tag: {0:08x?}")]
    UnknownPaletteTag(u32),
    #[error("unknown encoding: {0:08x?}")]
    UnknownEncoding(u32),
    #[error("c string error in {0}: {1}")]
    CStringError(&'static str, std::ffi::FromVecWithNulError),
    #[error("utf8 error in {0}: {1}")]
    Utf8Error(&'static str, std::str::Utf8Error),
    #[error("utf16 error in {0}: {1}")]
    Utf16Error(&'static str, std::string::FromUtf16Error),
}

pub fn read<R>(mut input: R) -> Result<Vec<FileData>, ReadError>
where
    R: Read,
{
    let mut magic = [0; 8];
    input.read_exact(&mut magic)?;

    if magic != MAGIC {
        return Err(ReadError::UnexpectedMagic(magic));
    }

    let tvg_version = input.read_u32::<LE>()?;
    if tvg_version != TVG_VERSION {
        return Err(ReadError::UnexpectedVersion(tvg_version));
    }

    let thing_1 = input.read_u32::<LE>()?;
    let thing_2 = input.read_u32::<LE>()?;
    if thing_1 != 2 || thing_2 != 1 {
        return Err(ReadError::UnknownMystery(format!(
            "unexpected mystery values after the TVG version: {}, {} (expected 2, 1)",
            thing_1, thing_2
        )));
    }

    let tags = read_tags(&mut input)?;

    Ok(tags)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u32)]
pub enum FileTag {
    /// `CERT`: contains a certificate unique to the license
    Cert = 0x43455254,
    /// `<NUL><NUL><NUL><NUL>`: contains the main drawing data
    MainData = 0x00000000,
    /// `ENDT`: purpose unclear. Does not appear to have any contents.
    Endt = 0x454e4454,
    /// `TVCI`: contains information about the software that created the file
    Tvci = 0x54564349,
    /// `CREA`: purpose unclear. Seems to always contain the number 2
    Crea = 0x43524541,
    /// `tUAA`: contents of the underlay art layer
    LayerUnderlay = 0x74554141,
    /// `tCAA`: contents of the color art layer
    LayerColor = 0x74434141,
    /// `tLAA`: contents of the line art layer
    LayerLine = 0x744c4141,
    /// `tOAA`: contents of the overlay art layer
    LayerOverlay = 0x744f4141,
    /// `TPAL`: contains the color palette
    Palette = 0x5450414c,
    /// `TTOC`: an index of offsets in the main data
    Ttoc = 0x54544f43,
    /// `SIGN`: some sort of file signature or checksum. Seems to always be 74 bytes long
    Sign = 0x5349474e,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u32)]
pub enum EncodingTag {
    /// `UNCO`: uncompressed data
    Unco = 0x554e434f,
    /// `ZLIB`: zlib-compressed data
    Zlib = 0x5a4c4942,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "content", rename_all = "snake_case"))]
pub enum FileData {
    Certificate(String),
    Signature(Vec<u8>),
    Crea(u32),
    Endt,
    Main(Vec<FileData>),
    MainOffsets(Vec<(FileTag, u32)>),
    Identity {
        device: String,
        software_name: String,
    },
    LayerUnderlay(LayerData),
    LayerColor(LayerData),
    LayerLine(LayerData),
    LayerOverlay(LayerData),
    Palette(PaletteData),
}

fn read_tags<R>(mut input: R) -> Result<Vec<FileData>, ReadError>
where
    R: Read,
{
    let mut tags = Vec::new();
    loop {
        match read_tag(&mut input) {
            Ok(tag) => tags.push(tag),
            Err(ReadError::Io(err)) if err.kind() == io::ErrorKind::UnexpectedEof => {
                // FIXME: distinguish unexpected EOF from EOF because the file ended
                break Ok(tags);
            }
            Err(e) => break Err(e),
        }
    }
}

fn read_tag<R>(mut input: R) -> Result<FileData, ReadError>
where
    R: Read,
{
    let tag = input.read_u32::<byteorder::BE>()?;

    match FileTag::try_from(tag) {
        Ok(FileTag::Cert) => {
            let len = input.read_u32::<LE>()?;
            let mut reader = (&mut input).take(len as u64);

            // mystery thing
            let thing = reader.read_u32::<LE>()?;
            if thing != 1 {
                return Err(ReadError::UnknownMystery(format!(
                    "unexpected CERT header bytes: {} (expected 1)",
                    thing
                )));
            }
            let cert_len = reader.read_u32::<LE>()?;

            let mut cert = Vec::new();
            cert.resize(cert_len as usize, 0);
            reader.read_exact(&mut cert)?;
            let cert = String::from_utf8(cert)
                .map_err(|e| ReadError::Utf8Error("certificate", e.utf8_error()))?;

            Ok(FileData::Certificate(cert))
        }
        Ok(FileTag::MainData) => {
            let data = read_encoded_data(&mut input)?;
            Ok(FileData::Main(read_tags(io::Cursor::new(data))?))
        }
        Ok(FileTag::Endt) => Ok(FileData::Endt),
        Ok(FileTag::Crea) => {
            let data = read_encoded_data(&mut input)?;
            let mut buf_read = io::BufReader::new(io::Cursor::new(data));
            let thing = buf_read.read_u32::<LE>()?;
            if thing != 2 {
                return Err(ReadError::UnknownMystery(format!(
                    "unexpected CREA value: {} (expected 2)",
                    thing
                )));
            }
            // TODO: check EOF?
            Ok(FileData::Crea(thing))
        }
        Ok(FileTag::Tvci) => {
            let data = read_encoded_data(&mut input)?;
            let mut buf_read = io::BufReader::new(io::Cursor::new(data));
            // skip 13 mystery bytes
            buf_read.read_exact(&mut [0; 13])?;

            let mut device = Vec::new();
            buf_read.read_until(0, &mut device)?;
            let mut name = Vec::new();
            buf_read.read_until(0, &mut name)?;

            let device = std::ffi::CString::from_vec_with_nul(device)
                .map_err(|e| ReadError::CStringError("tvci device", e))?
                .into_string()
                .map_err(|e| ReadError::Utf8Error("tvci device", e.utf8_error()))?;
            let name = std::ffi::CString::from_vec_with_nul(name)
                .map_err(|e| ReadError::CStringError("tvci software name", e))?
                .into_string()
                .map_err(|e| ReadError::Utf8Error("tvci software name", e.utf8_error()))?;

            // TODO: check EOF?
            Ok(FileData::Identity {
                device,
                software_name: name,
            })
        }
        Ok(FileTag::LayerUnderlay) => Ok(FileData::LayerUnderlay(read_layer_data(&mut input)?)),
        Ok(FileTag::LayerColor) => Ok(FileData::LayerColor(read_layer_data(&mut input)?)),
        Ok(FileTag::LayerLine) => Ok(FileData::LayerLine(read_layer_data(&mut input)?)),
        Ok(FileTag::LayerOverlay) => Ok(FileData::LayerOverlay(read_layer_data(&mut input)?)),
        Ok(FileTag::Palette) => Ok(FileData::Palette(read_palette_data(&mut input)?)),
        Ok(FileTag::Ttoc) => {
            let count = input.read_u32::<LE>()?;
            let mut offsets = Vec::new();
            for _ in 0..count {
                match FileTag::try_from(input.read_u32::<byteorder::BE>()?) {
                    Ok(tag) => {
                        let offset = input.read_u32::<LE>()?;
                        offsets.push((tag, offset));
                    }
                    Err(tag) => {
                        return Err(ReadError::UnknownFileTag(tag.number));
                    }
                }
            }

            // read 8 mystery bytes
            input.read_exact(&mut [0; 8])?;

            Ok(FileData::MainOffsets(offsets))
        }
        Ok(FileTag::Sign) => {
            // let's hope it's always 74 bytes!
            let mut data = [0; 74];
            input.read_exact(&mut data)?;
            Ok(FileData::Signature(data.into()))
        }
        Err(tag) => Err(ReadError::UnknownFileTag(tag.number)),
    }
}
