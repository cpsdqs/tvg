use crate::read::{EncodingTag, ReadError};
use byteorder::{ReadBytesExt, LE};
use std::io::Read;

/// Reads encoded data into a buffer.
/// Encoded data starts with a tag describing the encoding ([EncodingTag]) and is followed by the
/// data length.
pub(crate) fn read_encoded_data<R>(mut input: R) -> Result<Vec<u8>, ReadError>
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

/// A Toon Boom quantized value.
///
/// # Format
/// Toon Boom Harmony uses a float-like fixed point format.
///
/// The fixed point is specified by a [POINT_QUANTUM], the smallest unit of resolution.
/// Toon Boom Harmony seems to always use a point quantum of 1 / 64, i.e. all values equal
/// n / 64 for some integer n.
///
/// Numbers are 32-bit values composed from an MSB 1 bit sign, an 8 bit exponent,
/// and a variable length fractional part, followed by zero bytes until the LSB.
///
/// The exponent specifies both the current range of values and the length of the fractional value.
///
/// | Exponent | Range         | Fract. Length |
/// | --------:| ------------- | ------------- |
/// |     0x79 | [1/64,  2/64[ | 0 bits        |
/// |     0x7A | [2/64,  4/64[ | 1 bit         |
/// |     0x7B | [4/64,  8/64[ | 2 bits        |
/// |     0x7C | [8/64, 16/64[ | 3 bits        |
/// |      ... | ...           | ...           |
/// |     0x80 | [   2,     4[ | 7 bits        |
/// |      ... | ...           | ...           |
///
/// The lower bound of the range ("base value") is `2 ^ (exponent - 0x7F)`.
/// The length of the fractional part is `exponent - 0x79`.
///
/// To obtain the combined value, the fractional part is *added* to the base value.
/// Thus, the absolute value is `2 ^ (exponent - 0x7F) + fractional / 64`.
/// The sign bit simply negates this value when it's set to 1.
///
/// As a special case, 0 is always encoded as all zeros.
#[derive(Clone, Copy, PartialEq)]
pub struct TbQuant {
    neg: bool,
    exp: u16,
    frac: u32,
}

impl TbQuant {
    /// Inverse point quantum.
    pub const INV_POINT_QUANTUM: f64 = 64.;

    /// Point quantum: the smallest unit of resolution for coordinates.
    pub const POINT_QUANTUM: f64 = 1. / Self::INV_POINT_QUANTUM;

    pub const ZERO: Self = TbQuant {
        neg: false,
        exp: 0,
        frac: 0,
    };

    pub fn decode(value: u32) -> Self {
        if value == 0 {
            return Self::ZERO;
        }
        let negative = value & 0x80_00_00_00 != 0;
        let exponent = (value & 0x7F_80_00_00) >> 23;
        let f = value & 0x00_7F_FF_FF;
        let f_bits = exponent.saturating_sub(0x79);
        let frac = f >> 23_u32.saturating_sub(f_bits);

        TbQuant {
            neg: negative,
            exp: exponent as u16,
            frac,
        }
    }

    pub fn encode(&self) -> u32 {
        if self.exp == 0 && self.frac == 0 {
            return 0;
        }

        let sign = if self.neg { 0x80_00_00_00 } else { 0 };

        let exp = (self.exp as u32) << 23;
        let f_bits = self.exp.saturating_sub(0x79);
        let frac = self.frac << 23_u16.saturating_sub(f_bits);

        sign | exp | frac
    }

    pub fn from_f64(f: f64) -> Self {
        let neg = f.is_sign_negative();
        let abs_val = f.abs();
        let exp = (abs_val.log2() as i32) + 0x7f;

        let base_val = (2_f64).powi(exp - 0x7f);
        let frac_val = abs_val - base_val;
        let frac = (frac_val * Self::INV_POINT_QUANTUM) as u32;

        Self {
            neg,
            exp: exp as u16,
            frac,
        }
    }

    pub fn as_f64(&self) -> f64 {
        let base_val = (2_f64).powi(self.exp as i32 - 0x7f);
        let frac_val = self.frac as f64 * Self::POINT_QUANTUM;
        let abs_val = base_val + frac_val;

        if self.neg {
            -abs_val
        } else {
            abs_val
        }
    }
}

impl std::fmt::Debug for TbQuant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let float = self.as_f64();
        let int_part = float.trunc();
        let frac_part = float.fract().abs() * 1024.;
        write!(f, "{}r{}", int_part, frac_part)
    }
}

#[test]
fn test_tb_quant() {
    let value = 0xC49FEC00;
    let tbq = TbQuant::decode(value);
    let value2 = tbq.encode();
    assert_eq!(value, value2);
    let float = tbq.as_f64();
    let tbq2 = TbQuant::from_f64(float);
    assert_eq!(tbq, tbq2);
}

#[deprecated]
pub fn toon_boom_to_float(value: u32) -> f64 {
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

/// Contains byte data (with appropriate debug formatting).
#[derive(Clone)]
pub struct Bytes(pub Vec<u8>);

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut is_first = true;
        for byte in &self.0 {
            if is_first {
                is_first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{:02x?}", byte)?;
        }
        Ok(())
    }
}
