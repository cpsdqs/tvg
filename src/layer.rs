use crate::read::ReadError;
use crate::util::{read_encoded_data, Bytes, TbQuant};
use byteorder::{ReadBytesExt, LE};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::io::{self, Read};

#[derive(Debug, Clone)]
pub enum LayerData {
    Empty,
    Vector(Vec<VectorShape>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum ShapeType {
    Unknown0 = 0,
    Unknown1 = 1,
    Fill = 2,
    Stroke = 3,
    Line = 6,
    Unknown7 = 7,
}

#[derive(Debug, Clone)]
pub struct VectorShape {
    pub ty: ShapeType,
    pub components: Vec<ShapeComponent>,
}

#[derive(Debug, Clone)]
pub struct ShapeComponent {
    pub tags: Vec<ShapeComponentData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum ShapeComponentTag {
    /// `TGSD`: seems to contain metadata
    Tgsd = 0x54475344,
    /// `TGBP`: contains a Bézier path
    Tgbp = 0x54474250,
    /// `tGTB`: seems to be related to the pencil
    Tgtb = 0x74475442,
    /// `tGTI`: seems to be related to the pencil
    Tgti = 0x74475449,
}

#[derive(Debug, Clone)]
pub enum ShapeComponentData {
    Info(ComponentInfo),
    Path(Path),
    Tgtb(StrokeWeight),
    Tgti(Bytes),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ComponentType {
    Fill = 0,
    Unknown1 = 1,
    Stroke = 2,
    Pencil = 4,
}

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub ty: ComponentType,
    pub color_id: Option<u64>,
}

pub type Point = (TbQuant, TbQuant);

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Line(Point),
    Cubic(Point, Point, Point),
}

#[derive(Debug)]
enum PathSegmentType {
    Line,
    Cubic,
}

impl PathSegmentType {
    fn read<R>(mut input: R, points: u32) -> Result<Vec<PathSegmentType>, ReadError>
    where
        R: Read,
    {
        // curve instructions are encoded from LSB to MSB as a stream of little codes:
        // MSB 1001 0011 LSB -> read backwards: 1 1 001 001 (line, line, cubic, cubic)

        let mut current = input.read_u8()?;
        let mut pos = 0;

        let mut points_left = points;
        let mut out = Vec::with_capacity(points as usize / 3);
        let mut zeros = 0;
        while points_left > 0 {
            // read next bit
            let is_1 = {
                if pos > 7 {
                    current = input.read_u8()?;
                    pos -= 8;
                }
                let bit = (current & (1 << pos)) > 0;
                pos += 1;
                bit
            };

            if is_1 {
                match zeros {
                    0 => {
                        points_left -= 1;
                        out.push(PathSegmentType::Line);
                    }
                    2 => {
                        points_left -= 3;
                        out.push(PathSegmentType::Cubic);
                    }
                    n => {
                        return Err(ReadError::UnknownMystery(format!(
                            "unknown curve segment type {}",
                            n
                        )));
                    }
                }
                zeros = 0;
            } else {
                zeros += 1;
            }
        }
        Ok(out)
    }
}

impl Path {
    fn read<R>(mut input: R) -> Result<Self, ReadError>
    where
        R: Read,
    {
        let point_count = input.read_u32::<LE>()?;

        let segment_types = PathSegmentType::read(&mut input, point_count)?;
        let mut segments = Vec::new();

        macro_rules! read_point {
            () => {{
                let x = TbQuant::decode(input.read_u32::<LE>()?);
                let y = TbQuant::decode(input.read_u32::<LE>()?);
                (x, y)
            }};
        }

        for segment in segment_types {
            match segment {
                PathSegmentType::Line => {
                    segments.push(PathSegment::Line(read_point!()));
                }
                PathSegmentType::Cubic => {
                    segments.push(PathSegment::Cubic(
                        read_point!(),
                        read_point!(),
                        read_point!(),
                    ));
                }
            }
        }

        Ok(Path { segments })
    }
}

#[derive(Debug, Clone)]
pub struct StrokeWeight {
    points: Vec<StrokeWeightPoint>,
}

#[derive(Debug, Clone)]
pub struct StrokeWeightPoint {
    /// The location on the entire curve, from 0 to 1.
    location: TbQuant,
    /// The left weight side (in the drawing direction)
    left: StrokeWeightPointSide,
    /// The right weight side (in the drawing direction)
    right: StrokeWeightPointSide,
}

/// One side of a stroke weight point.
///
/// The offset and the Y coordinate of control points specify a distance from the center line.
///
/// The X coordinate of control points ranges from 0 to 1, where 0 is directly on the current weight
/// point and 1 is directly on the next weight point in that particular direction.
///
/// If this is an end point, however, things change:
/// the control point in the direction of the end cap (e.g. control_back at the beginning) is now
/// connected to the weight point on the other side of the *same* weight point.
/// The X coordinate is now 1 if the control point is on the other side of the stroke, and the Y
/// coordinate goes in the direction of the end cap.
#[derive(Debug, Clone)]
pub struct StrokeWeightPointSide {
    /// The offset from the center line.
    offset: TbQuant,
    /// The bézier control point in the backwards direction.
    control_back: Point,
    /// The bézier control point in the forwards direction.
    control_fwd: Point,
}

// what does this mean?
const LAYER_TRAILER: &[u8] = &[
    0x00, 0x54, 0x47, 0x52, 0x56, 0x08, 0x00, 0x00, 0x00, 0x3d, 0xdf, 0x4f, 0x8d,
];

pub fn read_layer_data<R>(mut input: R) -> Result<LayerData, ReadError>
where
    R: Read,
{
    let data = read_encoded_data(&mut input)?;
    let mut input = io::BufReader::new(io::Cursor::new(data));

    let data_type = input.read_u16::<LE>()?;
    match data_type {
        0 => {
            // empty layer
            Ok(LayerData::Empty)
        }
        0x0100 => {
            // vector layer
            read_vector_layer(input)
        }
        ty => Err(ReadError::UnknownMystery(format!(
            "unexpected value of layer data type: {:04x?}",
            ty
        ))),
    }
}

fn read_vector_layer<R>(mut input: R) -> Result<LayerData, ReadError>
where
    R: Read,
{
    let mut shapes = Vec::new();

    let shape_count = input.read_u32::<LE>()?;
    for i in 0..shape_count {
        let layer_ty = input.read_u32::<LE>()?;
        if layer_ty != 2 {
            return Err(ReadError::UnknownMystery(format!(
                "unexpected layer type: {:?}",
                layer_ty
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
                return Err(ReadError::UnknownShapeType(err.number));
            }
        };

        let mut paths = Vec::new();

        let component_count = input.read_u32::<LE>()?;
        for i in 0..component_count {
            let tag = input.read_u32::<byteorder::BE>()?;
            if tag != 0x54475653 {
                // not TGVS
                return Err(ReadError::UnknownMystery(format!(
                    "unexpected shape component tag: {:08x?}",
                    tag
                )));
            }

            let len = input.read_u32::<LE>()?;
            let mut input = (&mut input).take(len as u64);

            let mut tags = Vec::new();
            loop {
                let tag = match input.read_u32::<byteorder::BE>() {
                    Ok(tag) => match ShapeComponentTag::try_from(tag) {
                        Ok(tag) => tag,
                        Err(err) => return Err(ReadError::UnknownComponentTag(err.number)),
                    },
                    Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(err) => return Err(ReadError::Io(err)),
                };

                match tag {
                    ShapeComponentTag::Tgsd => {
                        let len = input.read_u32::<LE>()?;
                        {
                            let mut input = (&mut input).take(len as u64);

                            let component_type = ComponentType::try_from(input.read_u8()?)
                                .map_err(|err| ReadError::UnknownComponentType(err.number))?;

                            // TODO: find out what all the other stuff means (“TGCO”?)
                            let color_id = match component_type {
                                ComponentType::Fill => {
                                    // fill
                                    let color_id = match input.read_u8()? {
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
                                    };
                                    color_id
                                }
                                ComponentType::Unknown1 => None,
                                ComponentType::Stroke => {
                                    // stroke (the invisible kind)
                                    None
                                }
                                ComponentType::Pencil => {
                                    // pencil stroke
                                    input.read_u32::<LE>()?;
                                    Some(input.read_u64::<LE>()?)
                                }
                            };

                            // FIXME: is there any interesting data here, ever?
                            // seems to just be a bunch of 0 bytes, usually...
                            input.read_to_end(&mut Vec::new())?;

                            tags.push(ShapeComponentData::Info(ComponentInfo {
                                ty: component_type,
                                color_id,
                            }));
                        };

                        // for some reason, TGSD is always followed by an extra byte that indicates
                        // how to proceed
                        let extra_byte = input.read_u8()?;
                        match extra_byte {
                            0 => {
                                // stop
                                let trailer = input.read_u32::<LE>()?;
                                println!("trailer: {:08x?}", trailer);
                                break;
                            }
                            1 => {
                                // normal case: continue reading
                            }
                            n => {
                                return Err(ReadError::UnknownMystery(format!(
                                    "unexpected byte that follows TGSD: {:02x?}",
                                    n
                                )))
                            }
                        }
                    }
                    ShapeComponentTag::Tgbp => {
                        let len = input.read_u32::<LE>()?;
                        let mut input = (&mut input).take(len as u64);
                        tags.push(ShapeComponentData::Path(Path::read(&mut input)?));
                    }
                    ShapeComponentTag::Tgtb => {
                        let len = input.read_u32::<LE>()?;
                        let mut input = (&mut input).take(len as u64);

                        let header: [u8; 7] = [0x01, 0xff, 0xff, 0xff, 0xff, 0xcf, 0x00];
                        let mut header_read = [0; 7];
                        input.read_exact(&mut header_read)?;
                        if header != header_read {
                            return Err(ReadError::UnknownMystery(format!(
                                "unexpected tGTB header {:02x?}",
                                header_read,
                            )));
                        }

                        let mut point_count = input.read_u32::<LE>()?;
                        let mut points = Vec::new();

                        for _ in 0..point_count {
                            let loc = TbQuant::decode(input.read_u32::<LE>()?);
                            let off_l = TbQuant::decode(input.read_u32::<LE>()?);
                            let lb_x = TbQuant::decode(input.read_u32::<LE>()?);
                            let lb_y = TbQuant::decode(input.read_u32::<LE>()?);
                            let lf_x = TbQuant::decode(input.read_u32::<LE>()?);
                            let lf_y = TbQuant::decode(input.read_u32::<LE>()?);
                            let off_r = TbQuant::decode(input.read_u32::<LE>()?);
                            let rb_x = TbQuant::decode(input.read_u32::<LE>()?);
                            let rb_y = TbQuant::decode(input.read_u32::<LE>()?);
                            let rf_x = TbQuant::decode(input.read_u32::<LE>()?);
                            let rf_y = TbQuant::decode(input.read_u32::<LE>()?);

                            points.push(StrokeWeightPoint {
                                location: loc,
                                left: StrokeWeightPointSide {
                                    offset: off_l,
                                    control_back: (lb_x, lb_y),
                                    control_fwd: (lf_x, lf_y),
                                },
                                right: StrokeWeightPointSide {
                                    offset: off_r,
                                    control_back: (rb_x, rb_y),
                                    control_fwd: (rf_x, rf_y),
                                },
                            });
                        }

                        let trailer: [u8; 29] = [
                            00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
                            00, 0x80, 0x3F, 00, 00, 00, 00, 00, 00, 00, 00,
                        ];
                        let mut trailer_read = [0; 29];
                        input.read_exact(&mut trailer_read)?;
                        if trailer != trailer_read {
                            return Err(ReadError::UnknownMystery(format!(
                                "unexpected tGTB trailer {:02x?}",
                                trailer_read,
                            )));
                        }

                        tags.push(ShapeComponentData::Tgtb(StrokeWeight { points }));
                    }
                    ShapeComponentTag::Tgti => {
                        let len = input.read_u32::<LE>()?;
                        let mut input = (&mut input).take(len as u64);
                        // TODO
                        let mut data = Vec::new();
                        input.read_to_end(&mut data)?;
                        tags.push(ShapeComponentData::Tgti(Bytes(data)));
                    }
                }
            }

            paths.push(ShapeComponent { tags });
        }

        shapes.push(VectorShape {
            ty: shape_type,
            components: paths,
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

    Ok(LayerData::Vector(shapes))
}
