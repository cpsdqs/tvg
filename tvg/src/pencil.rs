use crate::layer::Point;
use crate::read::ReadError;
use crate::util::Bytes;
use byteorder::{ReadBytesExt, LE};
use std::io::Read;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StrokeThickness {
    /// Optional definition of a new thickness path.
    pub definition: Option<Vec<StrokeThicknessPoint>>,
    /// The domain of the thickness path that we're using for the current shape.
    pub domain: (f32, f32),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StrokeThicknessPoint {
    /// The location on the entire curve, from 0 to 1.
    pub loc: f32,
    /// The left thickness side (in the drawing direction)
    pub left: StrokeThicknessSide,
    /// The right thickness side (in the drawing direction)
    pub right: StrokeThicknessSide,
}

/// One side of a stroke thickness point.
///
/// The offset and the Y coordinate of control points specify a distance from the center line.
///
/// The X coordinate of control points ranges from 0 to 1, where 0 is directly on the current
/// thickness point and 1 is directly on the next thickness point in that particular direction.
///
/// If this is an end point, however, things change:
/// the control point in the direction of the end cap (e.g. control_back at the beginning) is now
/// connected to the thickness point on the other side of the *same* thickness point.
/// The X coordinate is now 1 if the control point is on the other side of the stroke, and the Y
/// coordinate goes in the direction of the end cap.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StrokeThicknessSide {
    /// The offset from the center line.
    pub offset: f32,
    /// The bézier control point in the backwards direction.
    ///
    /// The X value is in the (positive!) unit interval, while the Y value is an absolute offset.
    pub ctrl_back: Point,
    /// The bézier control point in the forwards direction.
    pub ctrl_fwd: Point,
}

/// The `tGTB` tag ends with information about the domain for the current shape component.
fn read_tgtb_domain(input: &mut impl Read) -> Result<(f32, f32), ReadError> {
    let domain_start = input.read_f32::<LE>()?;

    let unknown = input.read_u64::<LE>()?;
    if unknown != 0 {
        return Err(ReadError::UnknownMystery(format!(
            "unexpected tGTB bytes after domain start: {unknown:16x}",
        )));
    }

    let domain_end = input.read_f32::<LE>()?;

    let unknown = input.read_u64::<LE>()?;
    if unknown != 0 {
        return Err(ReadError::UnknownMystery(format!(
            "unexpected tGTB bytes after domain end: {unknown:16x}",
        )));
    }

    Ok((domain_start, domain_end))
}

/// Reads pencil thickness data in a `tGTB` tag.
pub fn read_tgtb(input: &mut impl Read) -> Result<StrokeThickness, ReadError> {
    let len = input.read_u32::<LE>()?;

    let mut input = input.take(len as u64);

    match input.read_u8()? {
        0x00 => {
            // uses a previously defined thickness path

            let header: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            let mut header_read = [0; 4];
            input.read_exact(&mut header_read)?;
            if header != header_read {
                let mut rest = Vec::new();
                input.read_to_end(&mut rest)?;

                return Err(ReadError::UnknownMystery(format!(
                    "unexpected tGTB use header: {:02x?} (rest: {:?})",
                    header_read,
                    Bytes(rest),
                )));
            }

            let domain = read_tgtb_domain(&mut input)?;

            Ok(StrokeThickness {
                definition: None,
                domain,
            })
        }
        0x01 => {
            // defines a thickness path

            let header: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xcf, 0x00];
            let mut header_read = [0; 6];
            input.read_exact(&mut header_read)?;
            if header != header_read {
                let mut rest = Vec::new();
                input.read_to_end(&mut rest)?;

                return Err(ReadError::UnknownMystery(format!(
                    "unexpected tGTB definition header: {:02x?} (rest: {:?})",
                    header_read,
                    Bytes(rest),
                )));
            }

            let point_count = input.read_u32::<LE>()?;
            let mut points = Vec::with_capacity(point_count as usize);

            for _ in 0..point_count {
                let loc = input.read_f32::<LE>()?;
                let off_l = input.read_f32::<LE>()?;
                let lb_x = input.read_f32::<LE>()?;
                let lb_y = input.read_f32::<LE>()?;
                let lf_x = input.read_f32::<LE>()?;
                let lf_y = input.read_f32::<LE>()?;
                let off_r = input.read_f32::<LE>()?;
                let rb_x = input.read_f32::<LE>()?;
                let rb_y = input.read_f32::<LE>()?;
                let rf_x = input.read_f32::<LE>()?;
                let rf_y = input.read_f32::<LE>()?;

                points.push(StrokeThicknessPoint {
                    loc,
                    left: StrokeThicknessSide {
                        offset: off_l,
                        ctrl_back: (lb_x, lb_y),
                        ctrl_fwd: (lf_x, lf_y),
                    },
                    right: StrokeThicknessSide {
                        offset: off_r,
                        ctrl_back: (rb_x, rb_y),
                        ctrl_fwd: (rf_x, rf_y),
                    },
                });
            }

            let trailer: [u8; 5] = [00, 00, 00, 00, 00];
            let mut trailer_read = [0; 5];
            input.read_exact(&mut trailer_read)?;
            if trailer != trailer_read {
                return Err(ReadError::UnknownMystery(format!(
                    "unexpected tGTB definition trailer: {:?}",
                    Bytes(trailer_read.into()),
                )));
            }

            let domain = read_tgtb_domain(&mut input)?;

            Ok(StrokeThickness {
                definition: Some(points),
                domain,
            })
        }
        byte => Err(ReadError::UnknownMystery(format!(
            "unknown tGTB type: {:02x?}",
            byte,
        ))),
    }
}
