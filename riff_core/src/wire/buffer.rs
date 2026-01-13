use crate::wire::constants::*;
use crate::wire::decode::{DecodeError, WireDecode};
use crate::wire::encode::WireEncode;

pub struct WireBuffer {
    data: Vec<u8>,
}

impl WireBuffer {
    pub fn new<T: WireEncode>(value: &T) -> Self {
        let content_size = value.wire_size();
        let total_size = HEADER_SIZE + content_size;

        let mut data = vec![0u8; total_size];

        data[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        data[4] = VERSION;
        data[5] = FLAGS_NONE;
        data[6..10].copy_from_slice(&(total_size as u32).to_le_bytes());

        value.encode_to(&mut data[HEADER_SIZE..]);

        Self { data }
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self, DecodeError> {
        let magic_bytes: [u8; 4] = data.get(0..4)
            .ok_or(DecodeError::BufferTooSmall)?
            .try_into()
            .map_err(|_| DecodeError::BufferTooSmall)?;
        let magic = u32::from_le_bytes(magic_bytes);
        if magic != MAGIC {
            return Err(DecodeError::InvalidMagic);
        }

        let version = *data.get(4).ok_or(DecodeError::BufferTooSmall)?;
        if version != VERSION {
            return Err(DecodeError::UnsupportedVersion);
        }

        Ok(Self { data })
    }

    pub fn validate(&self) -> Result<(), DecodeError> {
        let magic_bytes: [u8; 4] = self.data.get(0..4)
            .ok_or(DecodeError::BufferTooSmall)?
            .try_into()
            .map_err(|_| DecodeError::BufferTooSmall)?;
        let magic = u32::from_le_bytes(magic_bytes);
        if magic != MAGIC {
            return Err(DecodeError::InvalidMagic);
        }

        let version = *self.data.get(4).ok_or(DecodeError::BufferTooSmall)?;
        if version != VERSION {
            return Err(DecodeError::UnsupportedVersion);
        }

        let size_bytes: [u8; 4] = self.data.get(6..10)
            .ok_or(DecodeError::BufferTooSmall)?
            .try_into()
            .map_err(|_| DecodeError::BufferTooSmall)?;
        let claimed_size = u32::from_le_bytes(size_bytes) as usize;
        if claimed_size != self.data.len() {
            return Err(DecodeError::BufferTooSmall);
        }

        Ok(())
    }

    pub fn decode<T: WireDecode>(&self) -> Result<T, DecodeError> {
        self.validate()?;
        let (value, _) = T::decode_from(&self.data[HEADER_SIZE..])?;
        Ok(value)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn content_bytes(&self) -> &[u8] {
        &self.data[HEADER_SIZE..]
    }

    pub fn version(&self) -> u8 {
        self.data[4]
    }

    pub fn flags(&self) -> u8 {
        self.data[5]
    }

    pub fn total_size(&self) -> u32 {
        self.data.get(6..10)
            .and_then(|s| s.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
    }
}

impl AsRef<[u8]> for WireBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl From<WireBuffer> for Vec<u8> {
    fn from(buffer: WireBuffer) -> Self {
        buffer.data
    }
}

pub fn encode<T: WireEncode>(value: &T) -> Vec<u8> {
    WireBuffer::new(value).into_bytes()
}

pub fn decode<T: WireDecode>(data: &[u8]) -> Result<T, DecodeError> {
    let buffer = WireBuffer::from_bytes(data.to_vec())?;
    buffer.decode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_primitive() {
        let buffer = WireBuffer::new(&42i32);
        assert_eq!(buffer.len(), HEADER_SIZE + 4);

        let decoded: i32 = buffer.decode().unwrap();
        assert_eq!(decoded, 42);
    }

    #[test]
    fn buffer_string() {
        let original = "hello world".to_string();
        let buffer = WireBuffer::new(&original);

        let decoded: String = buffer.decode().unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn buffer_vec() {
        let original = vec![1i32, 2, 3, 4, 5];
        let buffer = WireBuffer::new(&original);

        let decoded: Vec<i32> = buffer.decode().unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn buffer_complex() {
        let original: Vec<Option<String>> = vec![
            Some("hello".to_string()),
            None,
            Some("world".to_string()),
        ];
        let buffer = WireBuffer::new(&original);

        let decoded: Vec<Option<String>> = buffer.decode().unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn buffer_validation() {
        let buffer = WireBuffer::new(&42i32);
        assert!(buffer.validate().is_ok());

        let bad_magic = vec![0, 0, 0, 0, 1, 0, 10, 0, 0, 0];
        assert!(WireBuffer::from_bytes(bad_magic).is_err());

        let bad_version = {
            let mut data = buffer.as_bytes().to_vec();
            data[4] = 99;
            data
        };
        let bad_buffer = WireBuffer { data: bad_version };
        assert!(bad_buffer.validate().is_err());
    }

    #[test]
    fn buffer_header_fields() {
        let buffer = WireBuffer::new(&42i32);

        assert_eq!(buffer.version(), VERSION);
        assert_eq!(buffer.flags(), FLAGS_NONE);
        assert_eq!(buffer.total_size() as usize, buffer.len());
    }

    #[test]
    fn encode_decode_helpers() {
        let original = vec!["hello".to_string(), "world".to_string()];
        let bytes = encode(&original);
        let decoded: Vec<String> = decode(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn nested_vec_roundtrip() {
        let original: Vec<Vec<i32>> = vec![
            vec![1, 2, 3],
            vec![4, 5],
            vec![6, 7, 8, 9],
        ];
        let buffer = WireBuffer::new(&original);
        let decoded: Vec<Vec<i32>> = buffer.decode().unwrap();
        assert_eq!(decoded, original);
    }
}
