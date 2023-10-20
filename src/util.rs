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
