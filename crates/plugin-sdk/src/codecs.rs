//! Desired custom image codecs. Register during preparation, before texture loading.
//! Native providers retain codec thunks until shutdown. Removing a rule revokes
//! decoding immediately; native registry slots are retired at provider shutdown.
use crate::{
    Context, Error,
    operations::{Operations, Registration},
};
/// Desired codec declarations: u8 extension length, u8 magic length, then both byte strings.
pub const CLAIMS: &str = "rekenshi.texture.codec-claims@1";
/// Decode copied encoded bytes; extension metadata; result width/height/Ogre format u32 then pixels.
pub const DECODE: &str = "rekenshi.texture.codec-decode@1";
/// Validated image with native-compatible uncompressed pixels; allocation stays local.
#[derive(Debug)]
pub struct Image {
    /// Positive width up to 32768.
    pub width: u32,
    /// Positive height up to 32768.
    pub height: u32,
    /// Ogre format: R8=1, R16=2, BGR8=10, BGRA8=12.
    pub format: u32,
    /// Exact tightly packed pixel bytes.
    pub pixels: Vec<u8>,
}
impl Image {
    fn validate(&self) -> Result<(), Error> {
        let channels = match self.format {
            1 => 1,
            2 => 2,
            10 => 3,
            12 => 4,
            _ => return Err("unsupported codec pixel format".into()),
        };
        let count = (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|n| n.checked_mul(channels))
            .ok_or("image size overflow")?;
        if self.width == 0
            || self.height == 0
            || self.width > 32768
            || self.height > 32768
            || count + 12 > 16 * 1024 * 1024
            || count != self.pixels.len()
        {
            return Err("invalid codec image size".into());
        }
        Ok(())
    }
    /// Copy a validated image into the provider wire schema.
    /// # Errors
    /// Rejects unsupported formats, dimensions or inconsistent pixel length.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut bytes = Vec::with_capacity(12 + self.pixels.len());
        for value in [self.width, self.height, self.format] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&self.pixels);
        Ok(bytes)
    }
    /// Validate a provider response before allocating native streams or metadata.
    /// # Errors
    /// Rejects malformed or excessive image records.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 12 || bytes.len() > 16 * 1024 * 1024 {
            return Err("invalid codec result".into());
        }
        let scalar = |i| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let image = Self {
            width: scalar(0),
            height: scalar(4),
            format: scalar(8),
            pixels: bytes[12..].to_vec(),
        };
        image.validate()?;
        Ok(image)
    }
}
/// Exclusive extension ownership and its DLL-local copied decoder callback.
#[derive(Debug)]
pub struct Codec {
    claim: Registration,
    decoder: Registration,
}
impl Codec {
    /// Declare a codec before the native provider activates. Late declarations require restart.
    /// # Errors
    /// Rejects invalid extensions/magic, duplicate extension owners, or host failure.
    pub fn new(
        context: &Context,
        extension: &str,
        magic: &[u8],
        mut decode: impl FnMut(&[u8]) -> Result<Image, Error> + Send + 'static,
    ) -> Result<Self, Error> {
        if extension.is_empty()
            || extension.len() > 15
            || !extension
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            || magic.is_empty()
            || magic.len() > 16
        {
            return Err("invalid codec declaration".into());
        }
        let operations = Operations::new(context)?;
        if operations
            .query(CLAIMS)
            .is_ok_and(|(_, status)| status.used != 0)
        {
            return Err("codec registration is late; restart required".into());
        }
        let name = extension.as_bytes().to_vec();
        let decoder = operations.transform(
            DECODE,
            &format!("rekenshi.codec.{extension}@1"),
            0,
            move |metadata, input| {
                if metadata != name {
                    return Ok(None);
                }
                Ok(Some(decode(input)?.encode()?))
            },
        )?;
        let mut declaration = vec![extension.len() as u8, magic.len() as u8];
        declaration.extend_from_slice(extension.as_bytes());
        declaration.extend_from_slice(magic);
        let claim = operations.transform(
            CLAIMS,
            &format!("rekenshi.codec-claim.{extension}@1"),
            0,
            move |_, input| Ok(Some([input, declaration.as_slice()].concat())),
        )?;
        Ok(Self { claim, decoder })
    }
    /// Quiesce decoding before releasing captured plugin state.
    /// # Errors
    /// A host cleanup failure retains ownership.
    pub fn remove(&mut self) -> Result<(), Error> {
        self.decoder.remove()?;
        self.claim.remove()
    }
}
/// Parse bounded native registration declarations.
/// # Errors
/// Rejects invalid declarations, duplicate extensions or more than 64 codecs.
pub fn declarations(mut bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut result = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 2 || result.len() == 64 {
            return Err("invalid codec list".into());
        }
        let (extension, magic) = (bytes[0] as usize, bytes[1] as usize);
        bytes = &bytes[2..];
        if !(1..=15).contains(&extension)
            || !(1..=16).contains(&magic)
            || bytes.len() < extension + magic
        {
            return Err("invalid codec declaration".into());
        }
        let name = std::str::from_utf8(&bytes[..extension])
            .map_err(|_| "invalid codec extension encoding")?;
        if !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            || result.iter().any(|(existing, _)| existing == name)
        {
            return Err("duplicate or invalid codec extension".into());
        }
        result.push((
            name.to_owned(),
            bytes[extension..extension + magic].to_vec(),
        ));
        bytes = &bytes[extension + magic..];
    }
    Ok(result)
}
