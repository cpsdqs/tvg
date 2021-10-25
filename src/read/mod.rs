use byteorder::{ReadBytesExt, LE};
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
    #[error("unknown path tag: {0:08x?}")]
    UnknownPathTag(u32),
    #[error("unknown palette tag: {0:08x?}")]
    UnknownPaletteTag(u32),
    #[error("unknown encoding: {0:08x?}")]
    UnknownEncoding(u32),
    #[error("c string error in {0}: {1}")]
    CStringError(&'static str, std::ffi::NulError),
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
#[repr(u32)]
pub enum FileTag {
    /// `CERT`: contains a certificate unique to the account
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
#[repr(u32)]
pub enum EncodingTag {
    /// `UNCO`: uncompressed data
    Unco = 0x554e434f,
    /// `ZLIB`: zlib-compressed data
    Zlib = 0x5a4c4942,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum PaletteTag {
    /// `TCSC`: contains the color value
    Tcsc = 0x54435343,
    /// `TCID`: contains information about the color (name, ID, project name)
    ColorId = 0x54434944,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum LayerData {
    Empty,
    Vector(Vec<VectorShape>),
}
#[derive(Debug, Clone)]
pub struct PaletteData {
    colors: Vec<PaletteColor>,
}

#[derive(Debug, Clone)]
pub struct PaletteColor {
    tags: Vec<ColorData>,
}

#[derive(Debug, Clone)]
pub enum ColorData {
    ColorRgba(u8, u8, u8, u8),
    ColorId {
        id: u64,
        name: String,
        project: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u16)]
enum ShapeType {
    Fill = 2,
    Stroke = 3,
    Line = 6,
}

#[derive(Debug, Clone)]
pub struct VectorShape {
    ty: ShapeType,
    paths: Vec<Path>,
}

#[derive(Debug, Clone)]
pub struct Path {
    tags: Vec<PathData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum PathTag {
    /// `TGSD`: seems to contain metadata
    Tgsd = 0x54475344,
    /// `TGBP`: contains a Bézier path
    Tgbp = 0x54474250,
    /// `TGTB`: seems to be related to the pencil
    Tgtb = 0x74475442,
    /// `TGTI`: seems to be related to the pencil
    Tgti = 0x74475449,
}

#[derive(Clone)]
pub struct Bytes(Vec<u8>);

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x?}", byte)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum PathData {
    Info(PathInfo),
    Path(PathCurve),
    Tgtb(Bytes),
    Tgti(Bytes),
}

#[derive(Debug, Clone)]
pub struct PathInfo {
    color_id: Option<u64>,
}

pub type Point = (f64, f64);

#[derive(Debug, Clone)]
pub enum PathCurve {
    Line(Point, Point),
    CubicBezier(Point, Point, Point, Point),
    PolyCubicBezier(Point, Vec<(Point, Point, Point)>),
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

            // FIXME: from_vec_with_nul is still unstable, so we'll pop the nul byte for now..
            device.pop();
            name.pop();

            let device = std::ffi::CString::new(device)
                .map_err(|e| ReadError::CStringError("tvci device", e))?
                .into_string()
                .map_err(|e| ReadError::Utf8Error("tvci device", e.utf8_error()))?;
            let name = std::ffi::CString::new(name)
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

fn read_encoded_data<R>(mut input: R) -> Result<Vec<u8>, ReadError>
where
    R: Read,
{
    let encoding_tag = input.read_u32::<byteorder::BE>()?;
    match EncodingTag::try_from(encoding_tag) {
        Ok(EncodingTag::Unco) => {
            let len = input.read_u32::<LE>()?;
            let mut data = Vec::new();
            data.resize(len as usize, 0);
            input.read_exact(&mut data)?;
            Ok(data)
        }
        Ok(EncodingTag::Zlib) => {
            let len = input.read_u32::<LE>()?;
            let decompressed_len = input.read_u32::<LE>()?;

            let mut decoder =
                libflate::zlib::Decoder::new((&mut input).take(len.saturating_sub(4) as u64))?;
            let mut data = Vec::with_capacity(decompressed_len as usize);
            decoder.read_to_end(&mut data)?;
            Ok(data)
        }
        Err(tag) => Err(ReadError::UnknownEncoding(tag.number)),
    }
}

const LAYER_TRAILER: &[u8] = &[
    0x00, 0x54, 0x47, 0x52, 0x56, 0x08, 0x00, 0x00, 0x00, 0x3d, 0xdf, 0x4f, 0x8d,
];

fn toon_boom_to_float(value: u32) -> f64 {
    if value == 0 {
        return 0.;
    }
    let negative = value & 0x80_00_00_00 != 0;
    let exponent = (value & 0x7F_80_00_00) >> 23;
    let f = value & 0x00_7F_FF_FF;
    let f_bits = exponent.saturating_sub(0x79);
    let base_val = (2_f64).powi(exponent as i32 - 0x7f);
    let frac_val = (f >> 23_u32.saturating_sub(f_bits)) as f64 / 64.;
    let abs_val = base_val + frac_val;
    if negative {
        -abs_val
    } else {
        abs_val
    }
}

fn read_layer_data<R>(mut input: R) -> Result<LayerData, ReadError>
where
    R: Read,
{
    let data = read_encoded_data(&mut input)?;
    let mut input = io::BufReader::new(io::Cursor::new(data));

    let data_type = input.read_u16::<LE>()?;
    match data_type {
        0 => {
            // empty layer
            return Ok(LayerData::Empty);
        }
        0x0100 => {
            // vector layer
        }
        ty => {
            return Err(ReadError::UnknownMystery(format!(
                "unexpected value of layer data type: {:04x?}",
                ty
            )));
        }
    }

    let mut layers = Vec::new();

    let shape_count = input.read_u32::<LE>()?;
    for _ in 0..shape_count {
        let shape_ty = input.read_u32::<LE>()?;
        if shape_ty != 2 {
            return Err(ReadError::UnknownMystery(format!(
                "unexpected layer type: {:?}",
                shape_ty
            )));
        }
        let tgly = input.read_u32::<byteorder::BE>()?;
        if tgly != 0x54474c59 {
            return Err(ReadError::UnknownMystery(format!(
                "unexpected layer tag: {:08x?}",
                tgly
            )));
        }
        let shape_len = input.read_u32::<LE>()?;
        let mut input = (&mut input).take(shape_len as u64);

        let shape_type = match ShapeType::try_from(input.read_u16::<LE>()?) {
            Ok(ty) => ty,
            Err(err) => {
                let mut data = Vec::new();
                input.read_to_end(&mut data)?;
                println!("{:?}", Bytes(data));
                return Err(ReadError::UnknownShapeType(err.number))
            },
        };

        let mut paths = Vec::new();

        let path_count = input.read_u32::<LE>()?;
        for _ in 0..path_count {
            let tag = input.read_u32::<byteorder::BE>()?;
            if tag != 0x54475653 {
                // not TGVS
                return Err(ReadError::UnknownMystery(format!(
                    "unexpected shape path tag: {:08x?}",
                    tag
                )));
            }

            let len = input.read_u32::<LE>()?;
            let mut input = (&mut input).take(len as u64);

            let mut tags = Vec::new();
            loop {
                let tag = match input.read_u32::<byteorder::BE>() {
                    Ok(tag) => match PathTag::try_from(tag) {
                        Ok(tag) => tag,
                        Err(err) => return Err(ReadError::UnknownPathTag(err.number)),
                    },
                    Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(err) => return Err(ReadError::Io(err)),
                };

                match tag {
                    PathTag::Tgsd => {
                        let len = input.read_u32::<LE>()?;
                        // for some reason, TGSD is followed by an extra 0x01 byte, so we'll
                        // include it here.
                        let mut input = (&mut input).take(len as u64 + 1);

                        // TODO: find out what all the other stuff means (“TGCOB”?)
                        let color_id = match input.read_u8()? {
                            0x04 => {
                                // stroke
                                input.read_u32::<LE>()?;
                                Some(input.read_u64::<LE>()?)
                            }
                            0x00 => {
                                // fill
                                match input.read_u8()? {
                                    0x00 => None,
                                    0x01 => {
                                        let color_pos = len - 24;
                                        for _ in 2..color_pos {
                                            input.read_u8()?;
                                        }
                                        Some(input.read_u64::<LE>()?)
                                    }
                                    t => {
                                        return Err(ReadError::UnknownMystery(format!(
                                            "unexpected second TGSD byte after 0x00: {}",
                                            t
                                        )))
                                    }
                                }
                            }
                            t => {
                                return Err(ReadError::UnknownMystery(format!(
                                    "unexpected first TGSD byte: {}",
                                    t
                                )))
                            }
                        };

                        input.read_to_end(&mut Vec::new())?;

                        tags.push(PathData::Info(PathInfo { color_id }));
                    }
                    PathTag::Tgbp => {
                        let len = input.read_u32::<LE>()?;
                        let mut input = (&mut input).take(len as u64);

                        let point_count = input.read_u32::<LE>()?;

                        enum CurveType {
                            Line,
                            CubicBezier,
                            PolyCubicBezier,
                        }

                        let curve_type = match input.read_u8()? {
                            0x3 => CurveType::Line,
                            0x9 => CurveType::CubicBezier,
                            0x49 => {
                                // Polybézier
                                // there's weird data before the points of variable length.
                                // it looks something like `92 24 09` or `92 24 49 92`.
                                // we'll just skip it
                                let weird_data_len = len - 5 - point_count * 8;
                                for _ in 0..weird_data_len {
                                    input.read_u8()?;
                                }
                                CurveType::PolyCubicBezier
                            }
                            t => {
                                return Err(ReadError::UnknownMystery(format!(
                                    "unknown vector curve type {:02x?}",
                                    t
                                )))
                            }
                        };

                        let mut points = Vec::new();

                        for _ in 0..point_count {
                            let x = toon_boom_to_float(input.read_u32::<LE>()?);
                            let y = toon_boom_to_float(input.read_u32::<LE>()?);
                            points.push((x, y));
                        }

                        let curve = match curve_type {
                            CurveType::Line => {
                                if points.len() != 2 {
                                    return Err(ReadError::UnknownMystery(format!("expected line segment to have 2 points but got {} point(s)", points.len())));
                                }
                                PathCurve::Line(points[0], points[1])
                            }
                            CurveType::CubicBezier => {
                                if points.len() != 4 {
                                    return Err(ReadError::UnknownMystery(format!("expected cubic bézier segment to have 4 points but got {} point(s)", points.len())));
                                }
                                PathCurve::CubicBezier(points[0], points[1], points[2], points[3])
                            }
                            CurveType::PolyCubicBezier => {
                                if points.is_empty() || (points.len().saturating_sub(1)) % 3 != 0 {
                                    return Err(ReadError::UnknownMystery(format!("expected poly-cubic bézier segment to have 3n+1 points but got {} point(s)", points.len())));
                                }
                                let first = points[0];
                                let mut curves = Vec::new();
                                for i in 0..(points.len() - 1) / 3 {
                                    let a = points[i * 3 + 1];
                                    let b = points[i * 3 + 2];
                                    let c = points[i * 3 + 3];
                                    curves.push((a, b, c));
                                }
                                PathCurve::PolyCubicBezier(first, curves)
                            }
                        };

                        tags.push(PathData::Path(curve));
                    }
                    PathTag::Tgtb => {
                        let len = input.read_u32::<LE>()?;
                        let mut input = (&mut input).take(len as u64);
                        let mut data = Vec::new();
                        input.read_to_end(&mut data)?;
                        tags.push(PathData::Tgtb(Bytes(data)));
                    }
                    PathTag::Tgti => {
                        let len = input.read_u32::<LE>()?;
                        let mut input = (&mut input).take(len as u64);
                        let mut data = Vec::new();
                        input.read_to_end(&mut data)?;
                        tags.push(PathData::Tgtb(Bytes(data)));
                    }
                }
            }

            paths.push(Path { tags });
        }

        layers.push(VectorShape {
            ty: shape_type,
            paths,
        });
    }

    let mut trailer = [0; LAYER_TRAILER.len()];
    input.read_exact(&mut trailer)?;
    if trailer != LAYER_TRAILER {
        return Err(ReadError::UnknownMystery(format!(
            "unexpected layer trailer: {:02?}",
            trailer
        )));
    }

    Ok(LayerData::Vector(layers))
}

fn read_palette_data<R>(mut input: R) -> Result<PaletteData, ReadError>
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

            match PaletteTag::try_from(tag) {
                Ok(PaletteTag::Tcsc) => {
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
                Ok(PaletteTag::ColorId) => {
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
