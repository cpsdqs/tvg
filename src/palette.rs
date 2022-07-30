use crate::read::{ColorData, ReadError};
use crate::util::read_encoded_data;
use byteorder::{ReadBytesExt, LE};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::io;
use std::io::Read;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum PaletteColorTag {
    /// `TCSC`: contains the color value
    Tcsc = 0x54435343,
    /// `TCID`: contains information about the color (name, ID, project name)
    ColorId = 0x54434944,
}

#[derive(Debug, Clone)]
pub struct PaletteData {
    pub colors: Vec<PaletteColor>,
}

#[derive(Debug, Clone)]
pub struct PaletteColor {
    pub tags: Vec<ColorData>,
}

pub fn read_palette_data<R>(mut input: R) -> Result<PaletteData, ReadError>
where
    R: Read,
{
    let data = read_encoded_data(&mut input)?;
    let mut input = io::BufReader::new(io::Cursor::new(data));

    let color_count = input.read_u32::<LE>()?;

    let first_end_tag = input.read_u32::<LE>()?;
    if first_end_tag != 0x79 {
        return Err(ReadError::UnknownMystery(format!(
            "expected palette color to start with 0x79, but found {}",
            first_end_tag
        )));
    }

    let mut colors = Vec::new();
    for _ in 0..color_count {
        let mystery_header = input.read_u16::<LE>()?;
        if mystery_header != 0 {
            return Err(ReadError::UnknownMystery(format!(
                "expected palette color header to be 0, but found {}",
                mystery_header
            )));
        }

        let mut tags = Vec::new();

        loop {
            let tag = match input.read_u32::<byteorder::BE>() {
                // some sort of end tag?
                Ok(0x79_00_00_00) => break,
                Ok(tag) => tag,
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(ReadError::Io(err)),
            };

            match PaletteColorTag::try_from(tag) {
                Ok(PaletteColorTag::Tcsc) => {
                    let len = input.read_u32::<LE>()?;
                    if len != 4 {
                        return Err(ReadError::UnknownMystery(format!(
                            "expected palette color TCSC tag to have length 4, but found length {}",
                            len
                        )));
                    }
                    let r = input.read_u8()?;
                    let g = input.read_u8()?;
                    let b = input.read_u8()?;
                    let a = input.read_u8()?;

                    tags.push(ColorData::ColorRgba(r, g, b, a));
                }
                Ok(PaletteColorTag::ColorId) => {
                    let len = input.read_u32::<LE>()?;
                    let mut input = (&mut input).take(len as u64);
                    let name_chars = input.read_u32::<LE>()?;

                    let mut name = Vec::with_capacity(name_chars as usize);
                    for _ in 0..name_chars {
                        name.push(input.read_u16::<LE>()?);
                    }
                    let name = String::from_utf16(&name)
                        .map_err(|e| ReadError::Utf16Error("palette color name", e))?;

                    let color_id = input.read_u64::<LE>()?;

                    let proj_chars = input.read_u32::<LE>()?;
                    let mut project = Vec::with_capacity(proj_chars as usize);
                    for _ in 0..proj_chars {
                        project.push(input.read_u16::<LE>()?);
                    }
                    let project = String::from_utf16(&project)
                        .map_err(|e| ReadError::Utf16Error("palette color project name", e))?;

                    tags.push(ColorData::ColorId {
                        id: color_id,
                        name,
                        project,
                    });
                }
                Err(err) => {
                    return Err(ReadError::UnknownPaletteTag(err.number));
                }
            }
        }

        colors.push(PaletteColor { tags });
    }

    Ok(PaletteData { colors })
}
